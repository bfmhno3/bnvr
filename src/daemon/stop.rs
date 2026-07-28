use super::ipc;
use super::process;
use crate::paths;
use std::fs;
use std::thread;
use std::time::Duration;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let pid_path = paths::pid_file();
    if !pid_path.exists() {
        return Err("daemon is not running (no PID file)".into());
    }

    let pid: u32 = fs::read_to_string(&pid_path)?.trim().parse()?;

    if !process::is_alive(pid) {
        fs::remove_file(&pid_path)?;
        return Err(format!("daemon is not running (stale PID file, pid {pid})").into());
    }

    let request = ipc::Request {
        id: 1,
        method: "shutdown".to_string(),
        params: serde_json::Value::Null,
    };
    let response = ipc::send_request(&request)
        .await
        .map_err(|e| format!("failed to request daemon shutdown: {e}"))?;
    if let Some(error) = response.error {
        return Err(format!("failed to request daemon shutdown: {error}").into());
    }

    for _ in 0..30 {
        if !process::is_alive(pid) {
            let _ = fs::remove_file(&pid_path);
            println!("daemon stopped (pid {pid})");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err(format!("daemon (pid {pid}) did not exit within 3 seconds").into())
}
