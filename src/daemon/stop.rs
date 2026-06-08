use super::{paths, process};
use std::fs;
use std::thread;
use std::time::Duration;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let pid_path = paths::pid_file();
    if !pid_path.exists() {
        return Err("daemon is not running (no PID file)".into());
    }

    let pid: u32 = fs::read_to_string(&pid_path)?.trim().parse()?;

    if !process::is_alive(pid) {
        // Stale PID file
        fs::remove_file(&pid_path)?;
        return Err(format!("daemon is not running (stale PID file, pid {pid})").into());
    }

    process::send_shutdown_signal(pid)?;

    // Wait up to 3 seconds for the process to exit
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
