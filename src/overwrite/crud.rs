use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::paths;

pub struct PluginInfo {
    pub name: String,
    pub active: bool,
    pub has_venv: bool,
    pub has_entry: bool,
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

pub fn init(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    init_in(&paths::overwrite_dir(), name)
}

pub fn init_in(overwrite_dir: &std::path::Path, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = overwrite_dir.join(name);
    if dir.exists() {
        return Err(format!("plugin already exists: {name}").into());
    }

    fs::create_dir_all(&dir)?;

    let entry = dir.join("overwrite.py");
    fs::write(&entry, ENTRY_TEMPLATE)?;

    // Try to create venv with uv, fall back to python -m venv
    let venv_result = Command::new("uv")
        .args(["venv"])
        .current_dir(&dir)
        .status();

    match venv_result {
        Ok(status) if status.success() => {}
        _ => {
            // Fallback to python -m venv
            let fallback = Command::new("python")
                .args(["-m", "venv", ".venv"])
                .current_dir(&dir)
                .status();

            match fallback {
                Ok(status) if status.success() => {}
                _ => {
                    tracing::warn!(
                        "neither 'uv' nor 'python' available; venv not created for plugin '{}'",
                        name
                    );
                }
            }
        }
    }

    Ok(dir)
}

pub fn list() -> Result<Vec<PluginInfo>, Box<dyn std::error::Error>> {
    list_in(&paths::overwrite_dir())
}

pub fn list_in(overwrite_dir: &std::path::Path) -> Result<Vec<PluginInfo>, Box<dyn std::error::Error>> {
    let active = get_active_in(overwrite_dir);
    let mut result = Vec::new();

    let entries = match fs::read_dir(overwrite_dir) {
        Ok(e) => e,
        Err(_) => return Ok(result),
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        result.push(PluginInfo {
            name: name_str.to_string(),
            active: active.as_deref() == Some(&name_str),
            has_venv: dir.join(".venv").exists(),
            has_entry: dir.join("overwrite.py").exists(),
        });
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

pub fn get_active() -> Option<String> {
    get_active_in(&paths::overwrite_dir())
}

pub fn get_active_in(overwrite_dir: &std::path::Path) -> Option<String> {
    fs::read_to_string(overwrite_dir.join(".active"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn set_active(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    set_active_in(&paths::overwrite_dir(), name)
}

pub fn set_active_in(overwrite_dir: &std::path::Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = overwrite_dir.join(name);
    if !dir.exists() {
        return Err(format!("plugin not found: {name}").into());
    }
    fs::write(overwrite_dir.join(".active"), name)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(test_name: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("bnvr-test-overwrite-{test_name}"));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    #[test]
    fn test_init_creates_directory_and_entry() {
        let dir = setup("init-creates");
        init_in(&dir, "test-plugin").unwrap();

        let plugin = dir.join("test-plugin");
        assert!(plugin.exists());
        assert!(plugin.join("overwrite.py").exists());

        let content = fs::read_to_string(plugin.join("overwrite.py")).unwrap();
        assert!(content.contains("def preprocess"));
        assert!(content.contains("def postprocess"));
        assert!(content.contains("def on_node_switch"));
        assert!(content.contains("def on_network_dropped"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_init_rejects_duplicate() {
        let dir = setup("init-dup");
        init_in(&dir, "dup-plugin").unwrap();
        let result = init_in(&dir, "dup-plugin");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_list_empty() {
        let dir = setup("list-empty");
        let plugins = list_in(&dir).unwrap();
        assert!(plugins.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_list_multiple() {
        let dir = setup("list-multiple");
        init_in(&dir, "beta").unwrap();
        init_in(&dir, "alpha").unwrap();

        let plugins = list_in(&dir).unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "alpha");
        assert_eq!(plugins[1].name, "beta");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_set_active_and_get_active() {
        let dir = setup("set-active");
        init_in(&dir, "my-plugin").unwrap();

        assert!(get_active_in(&dir).is_none());

        set_active_in(&dir, "my-plugin").unwrap();
        assert_eq!(get_active_in(&dir).as_deref(), Some("my-plugin"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_set_active_rejects_missing() {
        let dir = setup("set-active-missing");
        let result = set_active_in(&dir, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_list_shows_active_marker() {
        let dir = setup("list-active");
        init_in(&dir, "p1").unwrap();
        init_in(&dir, "p2").unwrap();
        set_active_in(&dir, "p1").unwrap();

        let plugins = list_in(&dir).unwrap();
        let p1 = plugins.iter().find(|p| p.name == "p1").unwrap();
        let p2 = plugins.iter().find(|p| p.name == "p2").unwrap();
        assert!(p1.active);
        assert!(!p2.active);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_init_creates_entry_with_valid_python_syntax() {
        let dir = setup("init-syntax");
        init_in(&dir, "syntax-test").unwrap();

        let content = fs::read_to_string(dir.join("syntax-test").join("overwrite.py")).unwrap();
        assert!(content.contains("if __name__ == \"__main__\":"));
        assert!(content.contains("json.load(sys.stdin)"));
        assert!(content.contains("json.dumps"));
        let _ = fs::remove_dir_all(dir);
    }
}
