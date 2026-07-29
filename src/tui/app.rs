use serde::Deserialize;

use super::log_reader::LogReader;
use crate::daemon::ipc::{self, Request};
use crate::paths;

const MAX_LOG_LINES: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedArea {
    NodeList,
    LogView,
}

impl FocusedArea {
    pub fn next(self) -> Self {
        match self {
            Self::NodeList => Self::LogView,
            Self::LogView => Self::NodeList,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DaemonStatus {
    pub pid: u32,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileInfo {
    pub active: Option<String>,
    pub list: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KernelStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginInfo {
    pub active: Option<String>,
    pub list: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionStats {
    pub total: usize,
    pub upload_bytes: u64,
    pub download_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusSnapshot {
    pub daemon: DaemonStatus,
    pub profile: ProfileInfo,
    pub kernel: KernelStatus,
    pub plugin: PluginInfo,
    pub connections: ConnectionStats,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub delay: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodesSnapshot {
    pub nodes: Vec<NodeInfo>,
    pub current: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrafficSample {
    pub timestamp: u64,
    pub upload_bps: u64,
    pub download_bps: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrafficSnapshot {
    pub samples: Vec<TrafficSample>,
}

pub struct AppState {
    pub focused: FocusedArea,
    pub should_quit: bool,
    pub daemon_connected: bool,
    pub daemon_status: Option<DaemonStatus>,
    pub profile_info: Option<ProfileInfo>,
    pub kernel_status: Option<KernelStatus>,
    pub plugin_info: Option<PluginInfo>,
    pub connection_stats: Option<ConnectionStats>,
    pub nodes: Vec<NodeInfo>,
    pub current_node: Option<String>,
    pub selected_node_index: usize,
    pub node_scroll_offset: usize,
    pub traffic_samples: Vec<TrafficSample>,
    pub log_lines: Vec<String>,
    pub log_scroll_offset: usize,
    pub log_auto_scroll: bool,
    log_reader: LogReader,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            focused: FocusedArea::NodeList,
            should_quit: false,
            daemon_connected: false,
            daemon_status: None,
            profile_info: None,
            kernel_status: None,
            plugin_info: None,
            connection_stats: None,
            nodes: Vec::new(),
            current_node: None,
            selected_node_index: 0,
            node_scroll_offset: 0,
            traffic_samples: Vec::new(),
            log_lines: Vec::new(),
            log_scroll_offset: 0,
            log_auto_scroll: true,
            log_reader: LogReader::new(paths::log_dir().join("bnvr.log")),
        }
    }

    pub async fn refresh_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.refresh_status().await;
        let _ = self.refresh_nodes().await;
        let _ = self.refresh_traffic().await;
        let _ = self.refresh_logs();
        Ok(())
    }

    pub async fn refresh_status(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let value = self
            .call_daemon("tui.status", serde_json::Value::Null)
            .await?;
        let status: StatusSnapshot = serde_json::from_value(value)?;
        self.daemon_connected = true;
        self.daemon_status = Some(status.daemon);
        self.profile_info = Some(status.profile);
        self.kernel_status = Some(status.kernel);
        self.plugin_info = Some(status.plugin);
        self.connection_stats = Some(status.connections);
        Ok(())
    }

    pub async fn refresh_nodes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let value = self
            .call_daemon("tui.nodes", serde_json::Value::Null)
            .await?;
        let snapshot: NodesSnapshot = serde_json::from_value(value)?;
        self.daemon_connected = true;
        self.nodes = snapshot.nodes;
        self.current_node = snapshot.current;
        if self.selected_node_index >= self.nodes.len() {
            self.selected_node_index = self.nodes.len().saturating_sub(1);
        }
        Ok(())
    }

    pub async fn refresh_traffic(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let value = self
            .call_daemon("tui.traffic", serde_json::Value::Null)
            .await?;
        let snapshot: TrafficSnapshot = serde_json::from_value(value)?;
        self.daemon_connected = true;
        self.traffic_samples = snapshot.samples;
        Ok(())
    }

    pub fn refresh_logs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.log_lines.is_empty() {
            self.log_lines = self.log_reader.read_tail(MAX_LOG_LINES)?;
        }
        let mut lines = self.log_reader.read_new_lines()?;
        self.log_lines.append(&mut lines);
        if self.log_lines.len() > MAX_LOG_LINES {
            let remove_count = self.log_lines.len() - MAX_LOG_LINES;
            self.log_lines.drain(0..remove_count);
            self.log_scroll_offset = self.log_scroll_offset.saturating_sub(remove_count);
        }
        if self.log_auto_scroll {
            self.log_scroll_offset = self.log_lines.len().saturating_sub(1);
        }
        Ok(())
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn toggle_focus(&mut self) {
        self.focused = self.focused.next();
    }

    pub fn select_next_node(&mut self) {
        if !self.nodes.is_empty() {
            self.selected_node_index = (self.selected_node_index + 1).min(self.nodes.len() - 1);
        }
    }

    pub fn select_prev_node(&mut self) {
        self.selected_node_index = self.selected_node_index.saturating_sub(1);
    }

    pub fn selected_node_name(&self) -> Option<String> {
        self.nodes
            .get(self.selected_node_index)
            .map(|node| node.name.clone())
    }

    pub fn set_node_delay(&mut self, name: &str, delay: u32) {
        if let Some(node) = self.nodes.iter_mut().find(|node| node.name == name) {
            node.delay = Some(delay);
        }
    }

    pub fn scroll_logs_up(&mut self, amount: usize) {
        self.log_auto_scroll = false;
        self.log_scroll_offset = self.log_scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_logs_down(&mut self, amount: usize) {
        self.log_scroll_offset =
            (self.log_scroll_offset + amount).min(self.log_lines.len().saturating_sub(1));
    }

    pub fn jump_to_latest_log(&mut self) {
        self.log_auto_scroll = true;
        self.log_scroll_offset = self.log_lines.len().saturating_sub(1);
    }

    pub fn toggle_log_auto_scroll(&mut self) {
        self.log_auto_scroll = !self.log_auto_scroll;
        if self.log_auto_scroll {
            self.jump_to_latest_log();
        }
    }

    async fn call_daemon(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let request = Request {
            id: 1,
            method: method.to_string(),
            params,
        };
        match ipc::send_request(&request).await {
            Ok(response) => {
                if let Some(error) = response.error {
                    self.daemon_connected = false;
                    Err(error.into())
                } else {
                    Ok(response.result.unwrap_or(serde_json::Value::Null))
                }
            }
            Err(e) => {
                self.daemon_connected = false;
                Err(e)
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
