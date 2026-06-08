use super::{paths, process};
use std::fs;
use tracing::{error, info};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    paths::ensure_dirs()?;

    // Check if daemon is already running
    let pid_path = paths::pid_file();
    if pid_path.exists() {
        let pid: u32 = fs::read_to_string(&pid_path)?.trim().parse()?;
        if process::is_alive(pid) {
            return Err(format!("daemon already running (pid {pid})").into());
        }
        // Stale PID file, clean up
        fs::remove_file(&pid_path)?;
    }

    // Write PID file
    let pid = std::process::id();
    fs::write(&pid_path, pid.to_string())?;
    info!(pid, "daemon starting");

    // Spawn IPC listener as background task
    let ipc_handle = tokio::spawn(async {
        if let Err(e) = super::ipc::listen().await {
            error!("IPC listener failed: {e}");
        }
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("received ctrl-c, shutting down");

    ipc_handle.abort();

    // Clean up PID file
    let _ = fs::remove_file(&pid_path);
    info!("daemon stopped");

    Ok(())
}
