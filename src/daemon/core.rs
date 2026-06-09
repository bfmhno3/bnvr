use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::kernel::manage;
use crate::paths;

pub struct KernelManager {
    inner: Arc<Mutex<KernelState>>,
}

struct KernelState {
    child: Option<Child>,
    version: Option<String>,
    restart_on_crash: bool,
}

impl KernelManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(KernelState {
                child: None,
                version: None,
                restart_on_crash: true,
            })),
        }
    }

    /// Start the active kernel. Returns the PID if successful.
    pub async fn start(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let mut state = self.inner.lock().await;

        if let Some(ref mut child) = state.child {
            if let Some(pid) = child.id() {
                return Err(format!("kernel already running (pid {pid})").into());
            }
            // Child exited, clean up
            state.child = None;
        }

        let version = manage::read_active()
            .ok_or("no active kernel version set (use `bnvr kernel use <version>`)")?;

        let binary = paths::kernel_binary_path(&version);
        if !binary.exists() {
            return Err(format!("binary not found: {}", binary.display()).into());
        }

        let child = Command::new(&binary)
            .arg("-d")
            .arg(paths::bnvr_home())
            .spawn()?;

        let pid = child.id().ok_or("failed to get child PID")?;
        info!(pid, version = %version, "mihomo started");

        state.child = Some(child);
        state.version = Some(version);

        Ok(pid)
    }

    /// Stop the running kernel.
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut state = self.inner.lock().await;

        let child = state.child.as_mut().ok_or("kernel not running")?;
        child.kill().await?;
        state.child = None;
        info!("mihomo stopped");

        Ok(())
    }

    /// Get current status.
    pub async fn status(&self) -> KernelManagerStatus {
        let state = self.inner.lock().await;

        let pid = state.child.as_ref().and_then(|c| c.id());

        KernelManagerStatus {
            running: pid.is_some(),
            pid,
            version: state.version.clone(),
        }
    }

    /// Spawn a background monitor task that restarts mihomo on crash.
    pub fn spawn_monitor(&self) -> tokio::task::JoinHandle<()> {
        let inner = self.inner.clone();

        tokio::spawn(async move {
            loop {
                // Wait until we have a child
                let wait_result = {
                    let mut state = inner.lock().await;
                    match state.child.as_mut() {
                        Some(child) => Some(child.wait().await),
                        None => None,
                    }
                };

                let result = match wait_result {
                    Some(r) => r,
                    None => {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                let should_restart = match result {
                    Ok(status) => {
                        warn!("mihomo exited with status: {status}");
                        true
                    }
                    Err(e) => {
                        error!("failed to wait on mihomo: {e}");
                        true
                    }
                };

                // Clean up the dead child
                {
                    let mut state = inner.lock().await;
                    state.child = None;
                }

                if !should_restart {
                    break;
                }

                // Check if restart is enabled
                {
                    let state = inner.lock().await;
                    if !state.restart_on_crash {
                        info!("restart_on_crash disabled, not restarting");
                        break;
                    }
                }

                warn!("restarting mihomo in 3 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                let version_opt = {
                    let state = inner.lock().await;
                    state.version.clone()
                };

                let Some(version) = version_opt else {
                    break;
                };

                let binary = paths::kernel_binary_path(&version);
                if !binary.exists() {
                    error!("binary not found: {}", binary.display());
                    break;
                }

                match Command::new(&binary)
                    .arg("-d")
                    .arg(paths::bnvr_home())
                    .spawn()
                {
                    Ok(child) => {
                        let pid = child.id().unwrap_or(0);
                        info!(pid, "mihomo restarted");
                        let mut state = inner.lock().await;
                        state.child = Some(child);
                    }
                    Err(e) => {
                        error!("failed to restart mihomo: {e}");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        })
    }
}

pub struct KernelManagerStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kernel_manager_new_not_running() {
        let km = KernelManager::new();
        let status = km.status().await;
        assert!(!status.running);
        assert!(status.pid.is_none());
        assert!(status.version.is_none());
    }

    #[tokio::test]
    async fn test_kernel_manager_stop_when_not_running() {
        let km = KernelManager::new();
        let result = km.stop().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not running"));
    }

    #[tokio::test]
    async fn test_kernel_manager_start_returns_result() {
        let km = KernelManager::new();
        let result = km.start().await;
        // start() should return a Result -- either Ok(pid) if a kernel is
        // configured and the binary exists, or Err if not.  We just verify
        // the function is callable and doesn't panic.
        let _ = result;
    }
}
