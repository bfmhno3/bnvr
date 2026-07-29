use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::{kernel::manage, overwrite, paths, profile, utilities};

pub struct KernelManager {
    inner: Arc<Mutex<KernelState>>,
}

struct KernelState {
    child: Option<Child>,
    version: Option<String>,
    restart_on_crash: bool,
    desired_running: bool,
}

impl KernelManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(KernelState {
                child: None,
                version: None,
                restart_on_crash: true,
                desired_running: false,
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
        state.desired_running = true;

        Ok(pid)
    }

    /// Stop the running kernel.
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut child = {
            let mut state = self.inner.lock().await;
            state.desired_running = false;
            state.child.take().ok_or("kernel not running")?
        };

        child.kill().await?;
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
                let exited = {
                    let mut state = inner.lock().await;
                    match state.child.as_mut() {
                        Some(child) => match child.try_wait() {
                            Ok(Some(status)) => {
                                warn!("mihomo exited with status: {status}");
                                state.child = None;
                                true
                            }
                            Ok(None) => false,
                            Err(e) => {
                                error!("failed to poll mihomo: {e}");
                                state.child = None;
                                true
                            }
                        },
                        None => false,
                    }
                };

                if !exited {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }

                let version = {
                    let state = inner.lock().await;
                    if !state.desired_running || !state.restart_on_crash || state.child.is_some() {
                        continue;
                    }
                    match state.version.clone() {
                        Some(version) => version,
                        None => continue,
                    }
                };

                warn!("restarting mihomo in 3 seconds...");
                tokio::time::sleep(Duration::from_secs(3)).await;

                loop {
                    {
                        let state = inner.lock().await;
                        if !state.desired_running
                            || !state.restart_on_crash
                            || state.child.is_some()
                        {
                            break;
                        }
                    }

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
                        Ok(mut child) => {
                            let pid = child.id().unwrap_or(0);
                            info!(pid, version = %version, "mihomo restarted");
                            let mut state = inner.lock().await;
                            if state.desired_running && state.child.is_none() {
                                state.child = Some(child);
                            } else {
                                drop(state);
                                if let Err(e) = child.kill().await {
                                    error!(error = %e, version = %version, "failed to stop discarded mihomo restart");
                                }
                                let _ = child.wait().await;
                            }
                            break;
                        }
                        Err(e) => {
                            error!(error = %e, version = %version, "failed to restart mihomo");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        })
    }
}

impl Default for KernelManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct KernelManagerStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub version: Option<String>,
}

pub async fn start_health_monitor(_state: Arc<super::state::DaemonState>) {
    let client = reqwest::Client::builder()
        .user_agent("bnvr")
        .timeout(Duration::from_secs(5))
        .build();
    let Ok(client) = client else {
        warn!("health monitor disabled: failed to build HTTP client");
        return;
    };
    let mut was_alive = true;

    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let response = client.get("http://127.0.0.1:9090/proxies").send().await;
        let Ok(response) = response else {
            warn!("health monitor skipped: Mihomo API unavailable");
            continue;
        };
        let value = match response.json::<serde_json::Value>().await {
            Ok(value) => value,
            Err(e) => {
                warn!(error = %e, "health monitor skipped: invalid Mihomo response");
                continue;
            }
        };
        let alive = value
            .get("proxies")
            .and_then(|proxies| proxies.as_object())
            .and_then(|proxies| proxies.values().find_map(|proxy| proxy.get("alive")))
            .and_then(|alive| alive.as_bool())
            .unwrap_or(true);

        if was_alive
            && !alive
            && let Some(active_plugin) = overwrite::crud::get_active()
        {
            match overwrite::bridge::run_hook(
                &active_plugin,
                "on_network_dropped",
                serde_json::json!({}),
                serde_json::Value::Null,
            )
            .await
            {
                Ok(_) => {
                    warn!(plugin = %active_plugin, "on_network_dropped hook triggered due to unhealthy node")
                }
                Err(e) => {
                    error!(plugin = %active_plugin, error = %e, "on_network_dropped hook failed")
                }
            }
        }
        was_alive = alive;
    }
}

pub async fn start_auto_sync_scheduler(_state: Arc<super::state::DaemonState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        run_auto_sync_cycle().await;
    }
}

pub async fn run_auto_sync_cycle() {
    let now = profile::crud::now_secs();
    match profile::crud::list() {
        Ok(profiles) => {
            for item in profiles {
                let Some(auto_sync) = item.meta.auto_sync.as_deref() else {
                    continue;
                };
                let interval = match utilities::effective_auto_sync_duration(
                    auto_sync,
                    item.meta.timeout.as_deref(),
                ) {
                    Ok(interval) => interval.as_secs(),
                    Err(e) => {
                        error!(type = "profile", name = %item.name, error = %e, "invalid auto-sync interval");
                        continue;
                    }
                };
                let last_sync = item.meta.updated_at.unwrap_or(item.meta.created_at);
                if now.saturating_sub(last_sync) < interval {
                    continue;
                }
                let name = item.name;
                let timeout = item
                    .meta
                    .timeout
                    .as_deref()
                    .map(utilities::parse_duration)
                    .transpose()
                    .ok()
                    .flatten()
                    .unwrap_or(Duration::from_secs(30));
                info!(type = "profile", name = %name, "auto-sync triggered");
                tokio::spawn(async move {
                    match tokio::time::timeout(timeout, profile::sync::sync_one(&name)).await {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => {
                            error!(type = "profile", name = %name, error = %e, "auto-sync failed")
                        }
                        Err(_) => error!(type = "profile", name = %name, "auto-sync timed out"),
                    }
                });
            }
        }
        Err(e) => error!(error = %e, "failed to list profiles for auto-sync"),
    }

    match overwrite::crud::list() {
        Ok(plugins) => {
            for item in plugins {
                let Some(meta) = item.meta else {
                    continue;
                };
                let Some(auto_sync) = meta.auto_sync.as_deref() else {
                    continue;
                };
                let interval = match utilities::effective_auto_sync_duration(
                    auto_sync,
                    meta.timeout.as_deref(),
                ) {
                    Ok(interval) => interval.as_secs(),
                    Err(e) => {
                        error!(type = "overwrite", name = %item.username, error = %e, "invalid auto-sync interval");
                        continue;
                    }
                };
                let last_sync = meta.updated_at.unwrap_or(meta.created_at);
                if now.saturating_sub(last_sync) < interval {
                    continue;
                }
                let username = item.username;
                let timeout = meta
                    .timeout
                    .as_deref()
                    .map(utilities::parse_duration)
                    .transpose()
                    .ok()
                    .flatten()
                    .unwrap_or(Duration::from_secs(30));
                info!(type = "overwrite", name = %username, "auto-sync triggered");
                tokio::spawn(async move {
                    let task = tokio::task::spawn_blocking({
                        let username = username.clone();
                        move || overwrite::crud::update(&username).map_err(|e| e.to_string())
                    });
                    match tokio::time::timeout(timeout, task).await {
                        Ok(Ok(Ok(()))) => {}
                        Ok(Ok(Err(e))) => {
                            error!(type = "overwrite", name = %username, error = %e, "auto-sync failed")
                        }
                        Ok(Err(e)) => {
                            error!(type = "overwrite", name = %username, error = %e, "auto-sync task failed")
                        }
                        Err(_) => {
                            error!(type = "overwrite", name = %username, "auto-sync timed out")
                        }
                    }
                });
            }
        }
        Err(e) => error!(error = %e, "failed to list overwrite plugins for auto-sync"),
    }
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
        let _ = result;
    }

    #[tokio::test]
    async fn test_monitor_stop_does_not_deadlock_or_restart() {
        let km = KernelManager::new();
        let child = long_running_child();
        let pid = child.id().unwrap();
        {
            let mut state = km.inner.lock().await;
            state.child = Some(child);
            state.version = Some("test-version".to_string());
            state.desired_running = true;
        }

        let monitor = km.spawn_monitor();
        let status = tokio::time::timeout(Duration::from_secs(1), km.status())
            .await
            .unwrap();
        assert_eq!(status.pid, Some(pid));

        tokio::time::timeout(Duration::from_secs(1), km.stop())
            .await
            .unwrap()
            .unwrap();
        tokio::time::sleep(Duration::from_secs(4)).await;

        let status = km.status().await;
        assert!(!status.running);
        monitor.abort();
    }

    fn long_running_child() -> Child {
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "ping", "-t", "127.0.0.1"])
                .spawn()
                .unwrap()
        } else {
            Command::new("sleep").arg("30").spawn().unwrap()
        }
    }
}
