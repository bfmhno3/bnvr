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

pub fn init_in(
    overwrite_dir: &std::path::Path,
    name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    paths::validate_component(name, "plugin name")?;
    let dir = overwrite_dir.join(name);
    if dir.exists() {
        return Err(format!("plugin already exists: {name}").into());
    }

    fs::create_dir_all(&dir)?;

    let entry = dir.join("overwrite.py");
    fs::write(&entry, ENTRY_TEMPLATE)?;

    let venv_result = Command::new("uv").args(["venv"]).current_dir(&dir).status();

    match venv_result {
        Ok(status) if status.success() => {}
        _ => {
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

pub fn list_in(
    overwrite_dir: &std::path::Path,
) -> Result<Vec<PluginInfo>, Box<dyn std::error::Error>> {
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

pub fn set_active_in(
    overwrite_dir: &std::path::Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    paths::validate_component(name, "plugin name")?;
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
        assert!(plugin.join("overwrite.py").exists());
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
        fs::create_dir_all(dir.join("plugin-b")).unwrap();
        fs::create_dir_all(dir.join("plugin-a")).unwrap();
        fs::write(dir.join("plugin-a").join("overwrite.py"), "").unwrap();
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
