use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::paths;
use crate::profile::crud::{now_secs, write_atomic};
use crate::utilities::validate_auto_sync_timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Remote,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub kind: PluginKind,
    pub link: String,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_sync: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

pub struct PluginInfo {
    pub name: String,
    pub username: String,
    pub active: bool,
    pub has_venv: bool,
    pub has_entry: bool,
    pub kind: PluginKind,
    pub meta: Option<PluginMeta>,
}

const ENTRY_TEMPLATE: &str = r#"import sys
import json


def preprocess(config):
    """Runs before Mihomo receives the config."""
    return config


def postprocess(config):
    """Runs after Mihomo processes the config."""
    return config


def on_node_switch(config, node_name):
    """Runs when switching proxy nodes."""
    return config


def on_network_dropped(config):
    """Runs when network drops."""
    return config


if __name__ == "__main__":
    request = json.load(sys.stdin)
    hook = request["hook"]
    config = request["config"]
    extra = request.get("extra", {})

    handlers = {
        "preprocess": preprocess,
        "postprocess": postprocess,
        "on_node_switch": lambda c: on_node_switch(c, extra.get("node_name", "")),
        "on_network_dropped": on_network_dropped,
    }

    if hook not in handlers:
        print(json.dumps({"config": config, "error": f"unknown hook: {hook}"}))
        sys.exit(1)

    result = handlers[hook](config)
    print(json.dumps({"config": result}))
"#;

pub fn read_plugin_meta(username: &str) -> Result<PluginMeta, Box<dyn Error>> {
    let content = fs::read_to_string(paths::plugin_meta_file(username))?;
    serde_json::from_str(&content).map_err(Into::into)
}

pub fn write_plugin_meta(username: &str, meta: &PluginMeta) -> Result<(), Box<dyn Error>> {
    write_atomic(
        &paths::plugin_meta_file(username),
        &serde_json::to_string_pretty(meta)?,
    )?;
    Ok(())
}

pub fn init(username: &str) -> Result<PathBuf, Box<dyn Error>> {
    init_in(&paths::overwrite_dir(), username)
}

pub fn init_in(overwrite_dir: &Path, username: &str) -> Result<PathBuf, Box<dyn Error>> {
    paths::validate_component(username, "username")?;
    let dir = overwrite_dir.join(username);
    if dir.exists() {
        return Err(format!("plugin already exists: {username}").into());
    }

    fs::create_dir_all(&dir)?;
    let repo = dir.join("overwrite");
    fs::create_dir_all(&repo)?;
    fs::write(repo.join("overwrite.py"), ENTRY_TEMPLATE)?;

    let meta = PluginMeta {
        kind: PluginKind::Local,
        link: String::new(),
        created_at: now_secs(),
        updated_at: None,
        auto_sync: None,
        timeout: None,
    };
    write_atomic(
        &dir.join("meta.json"),
        &serde_json::to_string_pretty(&meta)?,
    )?;

    create_venv(&repo, username);

    Ok(dir)
}

pub fn add(
    username: &str,
    link: &str,
    kind: PluginKind,
    auto_sync: Option<&str>,
    timeout: Option<&str>,
) -> Result<PathBuf, Box<dyn Error>> {
    paths::validate_component(username, "username")?;
    validate_auto_sync_timeout(auto_sync, timeout)?;

    let dir = paths::plugin_dir(username);
    if dir.exists() {
        return Err(format!("plugin already exists: {username}").into());
    }
    fs::create_dir_all(&dir)?;
    let repo = paths::plugin_repo_dir(username);

    let resolved_link = match kind {
        PluginKind::Remote => {
            let url = if link.contains("://") {
                link.to_string()
            } else {
                format!("https://github.com/{link}")
            };
            let output = Command::new("git")
                .args(["clone", &url])
                .arg(&repo)
                .output();
            match output {
                Ok(output) if output.status.success() => {}
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let _ = fs::remove_dir_all(&dir);
                    return Err(format!("git clone failed: {}", stderr.trim()).into());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    let _ = fs::remove_dir_all(&dir);
                    return Err("git not found; install git to use remote plugins".into());
                }
                Err(e) => {
                    let _ = fs::remove_dir_all(&dir);
                    return Err(e.into());
                }
            }
            url
        }
        PluginKind::Local => {
            let source = PathBuf::from(link);
            if !source.is_dir() {
                let _ = fs::remove_dir_all(&dir);
                return Err(format!("local plugin directory not found: {link}").into());
            }
            fs::create_dir_all(&repo)?;
            let source_root = source.canonicalize()?;
            let destination_root = repo.canonicalize()?;
            if destination_root.starts_with(&source_root) {
                let _ = fs::remove_dir_all(&dir);
                return Err("local plugin destination cannot be inside source directory".into());
            }
            copy_dir_contents(&source, &repo)?;
            link.to_string()
        }
    };

    create_venv(&repo, username);

    let meta = PluginMeta {
        kind,
        link: resolved_link,
        created_at: now_secs(),
        updated_at: None,
        auto_sync: auto_sync.map(str::to_string),
        timeout: timeout.map(str::to_string),
    };
    write_plugin_meta(username, &meta)?;
    Ok(dir)
}

pub fn update(username: &str) -> Result<(), Box<dyn Error>> {
    paths::validate_component(username, "username")?;
    let mut meta = read_plugin_meta(username)?;
    match meta.kind {
        PluginKind::Remote => {
            let repo = paths::plugin_repo_dir(username);
            if !repo.exists() {
                fs::create_dir_all(paths::plugin_dir(username))?;
                let output = Command::new("git")
                    .args(["clone", &meta.link])
                    .arg(&repo)
                    .output()?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("git clone failed: {}", stderr.trim()).into());
                }
            } else {
                run_git_pull_with_timeout(&repo, meta.timeout.as_deref().unwrap_or("30s"))?;
            }
        }
        PluginKind::Local => return Err("local plugins cannot be updated via Git".into()),
    }
    meta.updated_at = Some(now_secs());
    write_plugin_meta(username, &meta)
}

pub fn remove(username: &str) -> Result<(), Box<dyn Error>> {
    paths::validate_component(username, "username")?;
    let dir = paths::plugin_dir(username);
    if !dir.exists() {
        return Err(format!("plugin not found: {username}").into());
    }
    fs::remove_dir_all(dir)?;
    if get_active().as_deref() == Some(username) {
        let _ = fs::remove_file(paths::active_plugin_file());
    }
    Ok(())
}

pub fn list() -> Result<Vec<PluginInfo>, Box<dyn Error>> {
    list_in(&paths::overwrite_dir())
}

pub fn list_in(overwrite_dir: &Path) -> Result<Vec<PluginInfo>, Box<dyn Error>> {
    let active = get_active_in(overwrite_dir);
    let mut result = Vec::new();

    let entries = match fs::read_dir(overwrite_dir) {
        Ok(e) => e,
        Err(_) => return Ok(result),
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let username = name.to_string_lossy();
        if username.starts_with('.') {
            continue;
        }
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let meta = fs::read_to_string(dir.join("meta.json"))
            .ok()
            .and_then(|content| serde_json::from_str::<PluginMeta>(&content).ok());
        let kind = meta
            .as_ref()
            .map(|meta| meta.kind)
            .unwrap_or(PluginKind::Local);
        result.push(PluginInfo {
            name: username.to_string(),
            username: username.to_string(),
            active: active.as_deref() == Some(&username),
            has_venv: dir.join("overwrite").join(".venv").exists(),
            has_entry: dir.join("overwrite").join("overwrite.py").exists(),
            kind,
            meta,
        });
    }

    result.sort_by(|a, b| a.username.cmp(&b.username));
    Ok(result)
}

pub fn get_active() -> Option<String> {
    get_active_in(&paths::overwrite_dir())
}

pub fn get_active_in(overwrite_dir: &Path) -> Option<String> {
    fs::read_to_string(overwrite_dir.join(".active"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn set_active(username: &str) -> Result<(), Box<dyn Error>> {
    set_active_in(&paths::overwrite_dir(), username)
}

pub fn set_active_in(overwrite_dir: &Path, username: &str) -> Result<(), Box<dyn Error>> {
    paths::validate_component(username, "username")?;
    let dir = overwrite_dir.join(username);
    if !dir.exists() {
        return Err(format!("plugin not found: {username}").into());
    }
    fs::write(overwrite_dir.join(".active"), username)?;
    Ok(())
}

fn create_venv(repo: &Path, username: &str) {
    let venv_result = Command::new("uv").args(["venv"]).current_dir(repo).status();

    match venv_result {
        Ok(status) if status.success() => {}
        _ => {
            let fallback = Command::new("python")
                .args(["-m", "venv", ".venv"])
                .current_dir(repo)
                .status();

            match fallback {
                Ok(status) if status.success() => {}
                _ => {
                    tracing::warn!(
                        "neither 'uv' nor 'python' available; venv not created for plugin '{}'",
                        username
                    );
                }
            }
        }
    }
}

fn copy_dir_contents(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() {
            fs::create_dir_all(&to)?;
            copy_dir_contents(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn run_git_pull_with_timeout(repo: &Path, timeout: &str) -> Result<(), Box<dyn Error>> {
    let timeout = crate::utilities::parse_duration(timeout).unwrap_or(Duration::from_secs(30));
    let mut child = Command::new("git")
        .args(["-C"])
        .arg(repo)
        .arg("pull")
        .spawn()?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            return Err(format!("git pull failed with status {status}").into());
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("git pull timed out after {timeout:?}").into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup(test_name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_init_creates_plugin_structure() {
        let dir = setup("bnvr_test_plugin_init");
        let plugin = init_in(&dir, "test-plugin").unwrap();
        assert!(plugin.exists());
        assert!(plugin.join("meta.json").exists());
        assert!(plugin.join("overwrite").join("overwrite.py").exists());
    }

    #[test]
    fn test_init_duplicate_fails() {
        let dir = setup("bnvr_test_plugin_duplicate");
        init_in(&dir, "test-plugin").unwrap();
        let result = init_in(&dir, "test-plugin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_list_empty() {
        let dir = setup("bnvr_test_plugin_list_empty");
        let plugins = list_in(&dir).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_list_plugins() {
        let dir = setup("bnvr_test_plugin_list");
        fs::create_dir_all(dir.join("plugin-b").join("overwrite")).unwrap();
        fs::create_dir_all(dir.join("plugin-a").join("overwrite")).unwrap();
        fs::write(
            dir.join("plugin-a").join("overwrite").join("overwrite.py"),
            "",
        )
        .unwrap();
        fs::write(dir.join(".active"), "plugin-b").unwrap();

        let plugins = list_in(&dir).unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "plugin-a");
        assert!(plugins[0].has_entry);
        assert_eq!(plugins[1].name, "plugin-b");
        assert!(plugins[1].active);
    }

    #[test]
    fn test_set_active() {
        let dir = setup("bnvr_test_plugin_active");
        fs::create_dir_all(dir.join("test-plugin")).unwrap();
        set_active_in(&dir, "test-plugin").unwrap();
        assert_eq!(get_active_in(&dir).unwrap(), "test-plugin");
    }

    #[test]
    fn test_set_active_missing_fails() {
        let dir = setup("bnvr_test_plugin_active_missing");
        let result = set_active_in(&dir, "missing");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_init_rejects_invalid_plugin_names_without_escape() {
        let dir = setup("bnvr_test_plugin_invalid_names");
        for name in ["", ".", "..", "../escape", "a/b", "a\\b"] {
            let result = init_in(&dir, name);
            assert!(result.is_err());
        }
        assert!(!dir.join("..").join("escape").exists());
        assert!(!std::env::temp_dir().join("escape").exists());
    }

    #[test]
    fn test_init_accepts_hyphenated_plugin_name() {
        let dir = setup("bnvr_test_plugin_valid_name");
        let plugin = init_in(&dir, "valid-plugin").unwrap();
        assert_eq!(plugin.file_name().unwrap(), "valid-plugin");
    }
}
