use bnvr::paths;
use bnvr::profile::{crud, merge, sync};
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn setup(test_name: &str) -> (PathBuf, std::sync::MutexGuard<'static, ()>) {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = std::env::temp_dir().join(format!("bnvr-test-profile-{test_name}"));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("profile")).unwrap();
    // SAFETY: serialized by ENV_LOCK, no concurrent env access
    unsafe { std::env::set_var("BNVR_HOME", &tmp) };
    (tmp, guard)
}

fn cleanup(tmp: &PathBuf) {
    let _ = fs::remove_dir_all(tmp);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_sync_writes_raw_yaml_to_profile_dir() {
    let (tmp, _guard) = setup("sync-writes-raw");
    let body = "proxies: []\n";
    let (url, _) = start_test_http_server(ok_response(body));
    crud::add("alpha", &url, None).unwrap();

    sync::sync_one("alpha").await.unwrap();

    assert_eq!(
        fs::read_to_string(paths::profile_raw_file("alpha")).unwrap(),
        body
    );
    let meta = crud::read_meta("alpha").unwrap();
    assert!(meta.updated_at.is_some());
    cleanup(&tmp);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_sync_sends_clash_user_agent() {
    let (tmp, _guard) = setup("sync-ua");
    let (url, request) = start_test_http_server(ok_response("proxies: []\n"));
    crud::add("alpha", &url, None).unwrap();

    sync::sync_one("alpha").await.unwrap();

    assert!(
        request
            .lock()
            .unwrap()
            .contains("user-agent: clash-verge/v")
    );
    cleanup(&tmp);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_sync_rejects_non_yaml_body() {
    let (tmp, _guard) = setup("sync-rejects-html");
    let (url, _) = start_test_http_server(ok_response("<html>login</html>"));
    crud::add("alpha", &url, None).unwrap();

    let err = sync::sync_one("alpha").await.unwrap_err();

    assert!(err.to_string().contains("expected a YAML mapping"));
    assert!(!paths::profile_raw_file("alpha").exists());
    cleanup(&tmp);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn test_use_materializes_mihomo_config() {
    let (tmp, _guard) = setup("use-materializes");
    let (url, _) = start_test_http_server(ok_response("proxies: []\n"));
    crud::add("alpha", &url, None).unwrap();
    sync::sync_one("alpha").await.unwrap();

    crud::activate("alpha").unwrap();

    let value: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(paths::mihomo_config_file()).unwrap()).unwrap();
    assert!(value.as_mapping().unwrap().contains_key("proxies"));
    assert_eq!(
        fs::read_to_string(paths::active_profile_file()).unwrap(),
        "alpha"
    );
    cleanup(&tmp);
}

#[test]
fn test_merge_writes_merge_profile() {
    let (tmp, _guard) = setup("merge-writes");
    crud::add("alpha", "http://example.com/a.yml", None).unwrap();
    crud::add("beta", "http://example.com/b.yml", None).unwrap();
    crud::write_atomic(&paths::profile_raw_file("alpha"), "proxies:\n  - {name: a1, type: ss, server: 1.1.1.1, port: 443, password: pw1}\n  - {name: a2, type: ss, server: 2.2.2.2, port: 443, password: pw2}\n").unwrap();
    crud::write_atomic(&paths::profile_raw_file("beta"), "proxies:\n  - {name: b1, type: ss, server: 1.1.1.1, port: 443, password: pw1}\n  - {name: b2, type: ss, server: 3.3.3.3, port: 443, password: pw3}\n").unwrap();

    merge::merge(&["alpha".to_string(), "beta".to_string()], None).unwrap();

    assert!(paths::profile_raw_file("alpha+beta").exists());
    let meta_json = fs::read_to_string(paths::profile_meta_file("alpha+beta")).unwrap();
    assert!(meta_json.contains("\"kind\": \"merge\""));
    let merged: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(paths::profile_raw_file("alpha+beta")).unwrap())
            .unwrap();
    assert_eq!(
        merged
            .as_mapping()
            .unwrap()
            .get("proxies")
            .unwrap()
            .as_sequence()
            .unwrap()
            .len(),
        3
    );
    cleanup(&tmp);
}

#[test]
fn test_del_removes_directory_and_clears_active() {
    let (tmp, _guard) = setup("del-removes");
    crud::add("alpha", "http://example.com/a.yml", None).unwrap();
    crud::write_atomic(&paths::profile_raw_file("alpha"), "proxies: []\n").unwrap();
    crud::activate("alpha").unwrap();

    crud::del("alpha").unwrap();

    assert!(!paths::profile_dir("alpha").exists());
    assert!(!paths::active_profile_file().exists());
    cleanup(&tmp);
}

fn ok_response(body: &'static str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn start_test_http_server(response: String) -> (String, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let request = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&request);
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).unwrap_or(0);
        *captured.lock().unwrap() = String::from_utf8_lossy(&buffer[..n]).to_string();
        stream.write_all(response.as_bytes()).unwrap();
    });
    (format!("http://{addr}/sub.yaml"), request)
}
