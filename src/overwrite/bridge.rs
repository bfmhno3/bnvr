use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{error, info};

use crate::paths;

#[derive(Debug, Serialize)]
pub struct HookRequest {
    pub hook: String,
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub extra: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct HookResponse {
    pub config: serde_json::Value,
    #[serde(default)]
    pub error: Option<String>,
}

const HOOK_TIMEOUT: Duration = Duration::from_secs(3);

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .output();
    }
}

pub async fn run_hook(
    plugin_name: &str,
    hook: &str,
    config: serde_json::Value,
    extra: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    run_hook_in(&paths::overwrite_dir(), plugin_name, hook, config, extra).await
}

pub async fn run_hook_in(
    overwrite_dir: &std::path::Path,
    plugin_name: &str,
    hook: &str,
    config: serde_json::Value,
    extra: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let plugin_dir = overwrite_dir.join(plugin_name);
    let venv = plugin_dir.join(".venv");
    let python = if cfg!(target_os = "windows") {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    };
    if !python.exists() {
        return Err(format!(
            "python not found for plugin '{}'; run `bnvr overwrite init {}` first",
            plugin_name, plugin_name
        )
        .into());
    }

    let entry = plugin_dir.join("overwrite.py");
    if !entry.exists() {
        return Err(format!("entry point not found: {}", entry.display()).into());
    }

    let request = HookRequest {
        hook: hook.to_string(),
        config,
        extra,
    };

    let request_json = serde_json::to_string(&request)?;

    let mut child = Command::new(&python)
        .arg(&entry)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    // Write request JSON to stdin and close it
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(request_json.as_bytes()).await?;
        stdin.shutdown().await?;
    }

    // Save PID for kill on timeout (wait_with_output takes ownership)
    let pid = child.id();

    // Wait with timeout
    let output = match tokio::time::timeout(HOOK_TIMEOUT, child.wait_with_output()).await {
        Ok(result) => result?,
        Err(_) => {
            // Timeout: kill the child process by PID
            error!(plugin = plugin_name, hook = hook, "hook timed out, killing process");
            if let Some(pid) = pid {
                kill_pid(pid);
            }
            return Err(format!("hook '{}' timed out after {:?}", hook, HOOK_TIMEOUT).into());
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "hook '{}' failed (exit {}): {}",
            hook,
            output.status.code().unwrap_or(-1),
            stderr.trim()
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let response: HookResponse = serde_json::from_str(stdout.trim())?;

    if let Some(ref err) = response.error {
        return Err(format!("hook '{}' returned error: {}", hook, err).into());
    }

    info!(plugin = plugin_name, hook = hook, "hook completed");
    Ok(response.config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_request_serialization() {
        let req = HookRequest {
            hook: "preprocess".to_string(),
            config: serde_json::json!({"port": 7890}),
            extra: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"hook\":\"preprocess\""));
        assert!(json.contains("\"config\""));
        // extra should be skipped when null
        assert!(!json.contains("extra"));
    }

    #[test]
    fn test_hook_request_serialization_with_extra() {
        let req = HookRequest {
            hook: "on_node_switch".to_string(),
            config: serde_json::json!({}),
            extra: serde_json::json!({"node_name": "jp-1"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("extra"));
        assert!(json.contains("jp-1"));
    }

    #[test]
    fn test_hook_response_deserialization() {
        let json = r#"{"config": {"port": 7890}}"#;
        let resp: HookResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.config["port"], 7890);
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_hook_response_deserialization_with_error() {
        let json = r#"{"config": {}, "error": "something went wrong"}"#;
        let resp: HookResponse = serde_json::from_str(json).unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap(), "something went wrong");
    }

    #[tokio::test]
    async fn test_run_hook_plugin_not_found() {
        let tmp = std::env::temp_dir().join("bnvr-test-bridge-notfound");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let result = run_hook_in(
            &tmp,
            "nonexistent",
            "preprocess",
            serde_json::json!({}),
            serde_json::Value::Null,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
        let _ = std::fs::remove_dir_all(tmp);
    }
}
