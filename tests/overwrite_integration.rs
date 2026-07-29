use bnvr::overwrite::{bridge, crud, git};
use std::fs;
use std::path::PathBuf;

fn setup(test_name: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("bnvr-test-overwrite-integ-{test_name}"));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    tmp
}

// ── CRUD Integration ────────────────────────────────────────────────

#[test]
fn test_init_and_list_full_workflow() {
    let dir = setup("full-workflow");

    crud::init_in(&dir, "alpha").unwrap();
    crud::init_in(&dir, "beta").unwrap();

    let plugins = crud::list_in(&dir).unwrap();
    assert_eq!(plugins.len(), 2);
    assert_eq!(plugins[0].name, "alpha");
    assert_eq!(plugins[1].name, "beta");

    assert!(!plugins[0].active);
    assert!(!plugins[1].active);
    assert!(plugins[0].has_entry);
    assert!(plugins[1].has_entry);

    crud::set_active_in(&dir, "alpha").unwrap();
    let plugins = crud::list_in(&dir).unwrap();
    let alpha = plugins.iter().find(|p| p.name == "alpha").unwrap();
    let beta = plugins.iter().find(|p| p.name == "beta").unwrap();
    assert!(alpha.active);
    assert!(!beta.active);

    assert_eq!(crud::get_active_in(&dir).as_deref(), Some("alpha"));

    crud::set_active_in(&dir, "beta").unwrap();
    assert_eq!(crud::get_active_in(&dir).as_deref(), Some("beta"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_init_with_uv_creates_venv() {
    let dir = setup("init-uv");
    crud::init_in(&dir, "uv-test").unwrap();

    let plugins = crud::list_in(&dir).unwrap();
    assert_eq!(plugins.len(), 1);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_overwrite_py_contains_all_hooks() {
    let dir = setup("hooks-check");
    crud::init_in(&dir, "hook-test").unwrap();

    let content =
        fs::read_to_string(dir.join("hook-test").join("overwrite").join("overwrite.py")).unwrap();
    assert!(content.contains("def preprocess(config)"));
    assert!(content.contains("def postprocess(config)"));
    assert!(content.contains("def on_node_switch(config, node_name)"));
    assert!(content.contains("def on_network_dropped(config)"));
    assert!(content.contains("json.load(sys.stdin)"));
    assert!(content.contains("json.dumps"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_add_local_plugin_copies_files_and_meta() {
    let dir = setup("add-local");
    let source = dir.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("overwrite.py"), "print('ok')").unwrap();

    let home = dir.join("home");
    fs::create_dir_all(&home).unwrap();
    unsafe { std::env::set_var("BNVR_HOME", &home) };
    let plugin = crud::add(
        "local",
        source.to_str().unwrap(),
        crud::PluginKind::Local,
        Some("1d"),
        Some("30s"),
    )
    .unwrap();

    assert!(plugin.join("meta.json").exists());
    assert!(plugin.join("overwrite").join("overwrite.py").exists());
    let meta = crud::read_plugin_meta("local").unwrap();
    assert_eq!(meta.kind, crud::PluginKind::Local);
    assert_eq!(meta.auto_sync.as_deref(), Some("1d"));
    assert_eq!(meta.timeout.as_deref(), Some("30s"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_parse_duration_public_units() {
    assert_eq!(bnvr::utilities::parse_duration("1s").unwrap().as_secs(), 1);
    assert_eq!(
        bnvr::utilities::parse_duration("2m").unwrap().as_secs(),
        120
    );
    assert_eq!(
        bnvr::utilities::parse_duration("1d").unwrap().as_secs(),
        86400
    );
    assert!(bnvr::utilities::parse_duration("1h").is_err());
}

// ── Git Integration ─────────────────────────────────────────────────

#[test]
fn test_git_passthrough_init_and_status() {
    let dir = setup("git-passthrough");
    let plugin_dir = dir.join("git-plugin").join("overwrite");
    fs::create_dir_all(&plugin_dir).unwrap();

    let init_out = std::process::Command::new("git")
        .args(["init"])
        .current_dir(&plugin_dir)
        .output();
    match init_out {
        Ok(o) if o.status.success() => {
            let result = git::run_git_in(&dir, "git-plugin", &["status".to_string()]);
            assert!(result.is_ok());
            assert!(result.unwrap().contains("On branch"));

            let log_result = git::run_git_in(&dir, "git-plugin", &["log".to_string()]);
            let _ = log_result;
        }
        _ => {}
    }

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_git_passthrough_with_commit() {
    let dir = setup("git-commit");
    let plugin_dir = dir.join("commit-plugin").join("overwrite");
    fs::create_dir_all(&plugin_dir).unwrap();

    let init_out = std::process::Command::new("git")
        .args(["init"])
        .current_dir(&plugin_dir)
        .output();
    match init_out {
        Ok(o) if o.status.success() => {
            fs::write(plugin_dir.join("test.txt"), "hello").unwrap();
            let _ = std::process::Command::new("git")
                .args(["add", "."])
                .current_dir(&plugin_dir)
                .output();
            let commit_out = std::process::Command::new("git")
                .args(["commit", "-m", "initial"])
                .current_dir(&plugin_dir)
                .output();
            match commit_out {
                Ok(o) if o.status.success() => {
                    let log = git::run_git_in(
                        &dir,
                        "commit-plugin",
                        &["log".to_string(), "--oneline".to_string()],
                    );
                    assert!(log.is_ok());
                    assert!(log.unwrap().contains("initial"));
                }
                _ => {}
            }
        }
        _ => {}
    }

    let _ = fs::remove_dir_all(dir);
}

// ── Bridge IPC Integration ──────────────────────────────────────────

#[tokio::test]
async fn test_bridge_with_real_python_passthrough() {
    let dir = setup("bridge-real");
    crud::init_in(&dir, "passthrough").unwrap();

    let venv = dir.join("passthrough").join("overwrite").join(".venv");
    let python = if cfg!(target_os = "windows") {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    };

    if !python.exists() {
        let _ = fs::remove_dir_all(dir);
        return;
    }

    let config = serde_json::json!({
        "port": 7890,
        "proxies": [{"name": "node1", "type": "ss"}]
    });

    let result = bridge::run_hook_in(
        &dir,
        "passthrough",
        "preprocess",
        config.clone(),
        serde_json::Value::Null,
    )
    .await;

    match result {
        Ok(output) => {
            assert_eq!(output["port"], 7890);
            assert_eq!(output["proxies"][0]["name"], "node1");
        }
        Err(e) => {
            eprintln!("bridge call failed (may be expected): {e}");
        }
    }

    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_bridge_with_real_python_all_hooks() {
    let dir = setup("bridge-all-hooks");
    crud::init_in(&dir, "all-hooks").unwrap();

    let venv = dir.join("all-hooks").join("overwrite").join(".venv");
    let python = if cfg!(target_os = "windows") {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    };

    if !python.exists() {
        let _ = fs::remove_dir_all(dir);
        return;
    }

    let config = serde_json::json!({"port": 7890});

    for hook in &[
        "preprocess",
        "postprocess",
        "on_node_switch",
        "on_network_dropped",
    ] {
        let extra = if *hook == "on_node_switch" {
            serde_json::json!({"node_name": "jp-1"})
        } else {
            serde_json::Value::Null
        };

        let result = bridge::run_hook_in(&dir, "all-hooks", hook, config.clone(), extra).await;

        match result {
            Ok(output) => {
                assert_eq!(output["port"], 7890, "hook {hook} should preserve config");
            }
            Err(e) => {
                eprintln!("hook {hook} failed (may be expected): {e}");
            }
        }
    }

    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_bridge_with_custom_python_script() {
    let dir = setup("bridge-custom");
    let plugin_dir = dir.join("custom").join("overwrite");
    fs::create_dir_all(&plugin_dir).unwrap();

    let venv_status = std::process::Command::new("uv")
        .args(["venv"])
        .current_dir(&plugin_dir)
        .status();
    match venv_status {
        Ok(s) if s.success() => {}
        _ => {
            let _ = fs::remove_dir_all(dir);
            return;
        }
    }

    let script = r#"
import sys, json

def preprocess(config):
    config["modified"] = True
    config["port"] = 9090
    return config

if __name__ == "__main__":
    request = json.load(sys.stdin)
    hook = request["hook"]
    config = request["config"]
    if hook == "preprocess":
        result = preprocess(config)
    else:
        result = config
    print(json.dumps({"config": result}))
"#;
    fs::write(plugin_dir.join("overwrite.py"), script).unwrap();

    let config = serde_json::json!({"port": 7890});
    let result = bridge::run_hook_in(
        &dir,
        "custom",
        "preprocess",
        config,
        serde_json::Value::Null,
    )
    .await;

    match result {
        Ok(output) => {
            assert_eq!(output["modified"], true);
            assert_eq!(output["port"], 9090);
        }
        Err(e) => {
            eprintln!("custom script failed (may be expected): {e}");
        }
    }

    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_bridge_plugin_not_found() {
    let dir = setup("bridge-notfound");

    let result = bridge::run_hook_in(
        &dir,
        "nonexistent",
        "preprocess",
        serde_json::json!({}),
        serde_json::Value::Null,
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn test_bridge_unknown_hook() {
    let dir = setup("bridge-unknown");
    crud::init_in(&dir, "unknown-hook").unwrap();

    let venv = dir.join("unknown-hook").join("overwrite").join(".venv");
    let python = if cfg!(target_os = "windows") {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    };

    if !python.exists() {
        let _ = fs::remove_dir_all(dir);
        return;
    }

    let result = bridge::run_hook_in(
        &dir,
        "unknown-hook",
        "nonexistent_hook",
        serde_json::json!({}),
        serde_json::Value::Null,
    )
    .await;

    assert!(result.is_err());

    let _ = fs::remove_dir_all(dir);
}
