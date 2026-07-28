use super::process;
use crate::paths;
use std::fs;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let pid_path = paths::pid_file();
    if !pid_path.exists() {
        println!("daemon is not running");
        return Ok(());
    }

    let pid: u32 = fs::read_to_string(&pid_path)?.trim().parse()?;

    if process::is_alive(pid) {
        println!("daemon is running (pid {pid})");
    } else {
        println!("daemon is not running (stale PID file, pid {pid})");
        fs::remove_file(&pid_path)?;
    }

    Ok(())
}
