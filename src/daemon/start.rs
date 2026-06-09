use crate::paths;
use super::core::KernelManager;
use super::process;
use std::fs;
use std::sync::Arc;
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

    // Create kernel manager
    let km = Arc::new(KernelManager::new());

    // Try to start the kernel (non-fatal if no kernel is configured)
    match km.start().await {
        Ok(kpid) => info!(kernel_pid = kpid, "kernel started"),
        Err(e) => info!("kernel not started: {e}"),
    }

    // Spawn kernel monitor
    let monitor_handle = km.spawn_monitor();

    // Spawn IPC listener as background task
    let km_for_ipc = km.clone();
    let ipc_handle = tokio::spawn(async move {
        if let Err(e) = super::ipc::listen_with_kernel(km_for_ipc).await {
            error!("IPC listener failed: {e}");
        }
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("received ctrl-c, shutting down");

    // Stop kernel before exiting
    if let Err(e) = km.stop().await {
        info!("kernel stop: {e}");
    }

    monitor_handle.abort();
    ipc_handle.abort();

    // Clean up PID file
    let _ = fs::remove_file(&pid_path);
    info!("daemon stopped");

    Ok(())
}
