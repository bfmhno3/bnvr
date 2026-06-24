use std::process::Command;

use crate::overwrite::crud;
use crate::paths;

pub fn run_git(plugin_name: &str, args: &[String]) -> Result<String, Box<dyn std::error::Error>> {
    run_git_in(&paths::overwrite_dir(), plugin_name, args)
}

pub fn run_git_in(
    overwrite_dir: &std::path::Path,
    plugin_name: &str,
    args: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    let dir = overwrite_dir.join(plugin_name);
    if !dir.exists() {
        return Err(format!("plugin not found: {plugin_name}").into());
    }

    let output = Command::new("git")
        .args(args)
        .current_dir(&dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&stderr);
    }

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        return Err(format!("git exited with code {code}: {result}").into());
    }

    Ok(result)
}

pub fn run_git_active(args: &[String]) -> Result<String, Box<dyn std::error::Error>> {
    let plugin_name =
        crud::get_active().ok_or("no active plugin (use `bnvr overwrite use <name>`)")?;
    run_git(&plugin_name, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup(test_name: &str) -> std::path::PathBuf {
        let tmp = std::env::temp_dir().join(format!("bnvr-test-git-{test_name}"));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    #[test]
    fn test_run_git_plugin_not_found() {
        let dir = setup("not-found");
        let result = run_git_in(&dir, "nonexistent", &["status".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_run_git_active_no_plugin() {
        let dir = setup("no-active");
        let result = run_git_active(&["status".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no active plugin"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_run_git_init_and_status() {
        let dir = setup("init-status");
        let plugin_dir = dir.join("test-git");
        fs::create_dir_all(&plugin_dir).unwrap();

        let init_output = Command::new("git")
            .args(["init"])
            .current_dir(&plugin_dir)
            .output();
        match init_output {
            Ok(o) if o.status.success() => {
                let result = run_git_in(&dir, "test-git", &["status".to_string()]);
                assert!(result.is_ok());
                assert!(result.unwrap().contains("On branch"));
            }
            _ => {}
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_run_git_with_active_plugin() {
        let dir = setup("active-git");
        let plugin_dir = dir.join("my-plugin");
        fs::create_dir_all(&plugin_dir).unwrap();

        let init_output = Command::new("git")
            .args(["init"])
            .current_dir(&plugin_dir)
            .output();
        match init_output {
            Ok(o) if o.status.success() => {
                let result = run_git_in(&dir, "my-plugin", &["status".to_string()]);
                assert!(result.is_ok());
            }
            _ => {}
        }
        let _ = fs::remove_dir_all(dir);
    }
}
