use super::core::KernelManager;
use super::process;
use super::state::DaemonState;
use crate::paths;
use std::fs;
use std::sync::Arc;
use tracing::{error, info};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    paths::ensure_dirs()?;

    let pid_path = paths::pid_file();
    if pid_path.exists() {
        let pid: u32 = fs::read_to_string(&pid_path)?.trim().parse()?;
        if process::is_alive(pid) {
            return Err(format!("daemon already running (pid {pid})").into());
        }
        fs::remove_file(&pid_path)?;
    }

    let pid = std::process::id();
    fs::write(&pid_path, pid.to_string())?;
    info!(pid, "daemon starting");

    let km = Arc::new(KernelManager::new());
    let daemon_state = Arc::new(DaemonState::new(km.clone()));

    match km.start().await {
        Ok(kpid) => info!(kernel_pid = kpid, "kernel started"),
        Err(e) => info!("kernel not started: {e}"),
    }

    let monitor_handle = km.spawn_monitor();
    let health_handle = tokio::spawn(super::core::start_health_monitor(daemon_state.clone()));
    let auto_sync_handle =
        tokio::spawn(super::core::start_auto_sync_scheduler(daemon_state.clone()));
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    let state_for_ipc = daemon_state.clone();
    let ipc_handle = tokio::spawn(async move {
        if let Err(e) = super::ipc::listen_with_state(state_for_ipc, shutdown_tx).await {
            error!("IPC listener failed: {e}");
        }
    });

    let shutdown_result: Result<(), Box<dyn std::error::Error>> = tokio::select! {
        result = tokio::signal::ctrl_c() => {
            result?;
            info!("received ctrl-c, shutting down");
            Ok(())
        }
        result = shutdown_rx.changed() => {
            result.map_err(|_| "IPC shutdown channel closed")?;
            if !*shutdown_rx.borrow() {
                Err("IPC shutdown channel changed without shutdown request".into())
            } else {
                info!("received IPC shutdown request, shutting down");
                Ok(())
            }
        }
    };

    if let Err(e) = daemon_state.tun.lock().await.clear() {
        info!("TUN cleanup: {e}");
    }

    if km.status().await.running
        && let Err(e) = km.stop().await
    {
        info!("kernel stop: {e}");
    }

    monitor_handle.abort();
    ipc_handle.abort();
    health_handle.abort();
    auto_sync_handle.abort();

    let _ = fs::remove_file(&pid_path);
    info!("daemon stopped");

    shutdown_result
}
