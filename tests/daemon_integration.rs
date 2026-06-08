use bnvr::daemon::{db, ipc};
use bnvr::paths;
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::{GenericNamespaced, ToNsName};
use interprocess::local_socket::traits::tokio::Stream as _;
use rusqlite::Connection;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

// Helper: start a test IPC listener on a unique socket name
fn start_test_listener(
    socket_name: &str,
) -> tokio::task::JoinHandle<Result<(), String>> {
    let name = socket_name.to_string();
    tokio::spawn(async move { ipc::listen_on(&name).await.map_err(|e| e.to_string()) })
}

// Helper: connect to a test socket and send a request, return the response
async fn send_test_request(
    socket_name: &str,
    request: &ipc::Request,
) -> Result<ipc::Response, String> {
    let ns_name = socket_name.to_ns_name::<GenericNamespaced>().map_err(|e| e.to_string())?;
    let mut stream = Stream::connect(ns_name).await.map_err(|e| e.to_string())?;

    let mut msg = serde_json::to_string(request).map_err(|e| e.to_string())?;
    msg.push('\n');
    stream.write_all(msg.as_bytes()).await.map_err(|e| e.to_string())?;

    let (recv_half, _) = stream.split();
    let mut reader = tokio::io::BufReader::new(recv_half);
    let mut line = String::new();
    reader.read_line(&mut line).await.map_err(|e| e.to_string())?;

    let resp: ipc::Response = serde_json::from_str(&line).map_err(|e| e.to_string())?;
    Ok(resp)
}

// ── IPC Integration Tests ────────────────────────────────────────────

#[tokio::test]
async fn test_ipc_status_roundtrip() {
    let socket_name = "bnvr_test_status";

    let _handle = start_test_listener(socket_name);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let req = ipc::Request {
        id: 1,
        method: "status".to_string(),
        params: serde_json::Value::Null,
    };

    let resp = send_test_request(socket_name, &req).await.unwrap();
    assert_eq!(resp.id, 1);
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    assert_eq!(result["status"], "running");
    assert!(result["pid"].as_u64().is_some());
}

#[tokio::test]
async fn test_ipc_unknown_method() {
    let socket_name = "bnvr_test_unknown";

    let _handle = start_test_listener(socket_name);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let req = ipc::Request {
        id: 2,
        method: "nonexistent".to_string(),
        params: serde_json::Value::Null,
    };

    let resp = send_test_request(socket_name, &req).await.unwrap();
    assert_eq!(resp.id, 2);
    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().contains("unknown method"));
}

#[tokio::test]
async fn test_ipc_invalid_json() {
    let socket_name = "bnvr_test_invalid_json";

    let _handle = start_test_listener(socket_name);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let ns_name = socket_name.to_ns_name::<GenericNamespaced>().unwrap();
    let mut stream = Stream::connect(ns_name).await.unwrap();

    stream.write_all(b"not valid json\n").await.unwrap();

    let (recv_half, _) = stream.split();
    let mut reader = tokio::io::BufReader::new(recv_half);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();

    let resp: ipc::Response = serde_json::from_str(&line).unwrap();
    assert_eq!(resp.id, 0);
    assert!(resp.error.is_some());
    assert!(resp.error.unwrap().contains("invalid request"));
}

#[tokio::test]
async fn test_ipc_multiple_requests_same_connection() {
    let socket_name = "bnvr_test_multi";

    let _handle = start_test_listener(socket_name);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let ns_name = socket_name.to_ns_name::<GenericNamespaced>().unwrap();
    let stream = Stream::connect(ns_name).await.unwrap();
    let (recv_half, mut send_half) = stream.split();
    let mut reader = tokio::io::BufReader::new(recv_half);

    for i in 1..=3 {
        let req = ipc::Request {
            id: i,
            method: "status".to_string(),
            params: serde_json::Value::Null,
        };
        let mut msg = serde_json::to_string(&req).unwrap();
        msg.push('\n');
        send_half.write_all(msg.as_bytes()).await.unwrap();

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let resp: ipc::Response = serde_json::from_str(&line).unwrap();
        assert_eq!(resp.id, i);
        assert!(resp.error.is_none());
        line.clear();
    }
}

#[tokio::test]
async fn test_ipc_multiple_concurrent_clients() {
    let socket_name = "bnvr_test_concurrent";

    let _handle = start_test_listener(socket_name);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut handles = Vec::new();
    for i in 1..=5 {
        let name = socket_name.to_string();
        handles.push(tokio::spawn(async move {
            let req = ipc::Request {
                id: i,
                method: "status".to_string(),
                params: serde_json::Value::Null,
            };
            send_test_request(&name, &req).await
        }));
    }

    for handle in handles {
        let resp = handle.await.unwrap().unwrap();
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }
}

// ── DB Integration Tests ─────────────────────────────────────────────

#[test]
fn test_db_open_creates_file() {
    let dir = std::env::temp_dir().join("bnvr_test_db_open");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let db_path = dir.join("test.db");
    let conn = Connection::open(&db_path).unwrap();
    db::init_schema(&conn).unwrap();

    assert!(db_path.exists());
    conn.close().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_db_full_workflow() {
    let conn = Connection::open_in_memory().unwrap();
    db::init_schema(&conn).unwrap();

    conn.execute(
        "INSERT INTO profiles (name, url) VALUES ('work', 'http://sub.example.com')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO subscriptions (profile_id, content) VALUES (1, 'proxies: []')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO audit_log (action, detail) VALUES ('profile_sync', 'work synced')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO bench_results (group_name, node, connect_ms, tls_ms, jitter_ms) VALUES ('asia', 'jp-1', 50.0, 120.0, 5.0)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO traffic_stats (domain, bytes_up, bytes_down) VALUES ('google.com', 1024, 2048)",
        [],
    )
    .unwrap();

    let profile_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))
        .unwrap();
    assert_eq!(profile_count, 1);

    let sub_content: String = conn
        .query_row(
            "SELECT s.content FROM subscriptions s JOIN profiles p ON s.profile_id = p.id WHERE p.name = 'work'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sub_content, "proxies: []");

    let audit_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
        .unwrap();
    assert_eq!(audit_count, 1);

    let bench_node: String = conn
        .query_row("SELECT node FROM bench_results WHERE id=1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(bench_node, "jp-1");

    let (up, down): (i64, i64) = conn
        .query_row(
            "SELECT bytes_up, bytes_down FROM traffic_stats WHERE domain='google.com'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(up, 1024);
    assert_eq!(down, 2048);
}

#[test]
fn test_db_delete_profile_cascades() {
    let conn = Connection::open_in_memory().unwrap();
    db::init_schema(&conn).unwrap();

    conn.execute("INSERT INTO profiles (name, url) VALUES ('p1', 'http://a.com')", [])
        .unwrap();
    conn.execute(
        "INSERT INTO subscriptions (profile_id, content) VALUES (1, 'data')",
        [],
    )
    .unwrap();

    conn.execute("DELETE FROM profiles WHERE id=1", []).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

// ── Paths Integration Tests ──────────────────────────────────────────

#[test]
fn test_ensure_dirs_creates_real_directories() {
    paths::ensure_dirs().unwrap();
    assert!(paths::bnvr_home().exists());
    assert!(paths::log_dir().exists());
}

// ── Process Integration Tests ────────────────────────────────────────

#[test]
fn test_process_current_pid_alive() {
    let pid = std::process::id();
    assert!(bnvr::daemon::process::is_alive(pid));
}
