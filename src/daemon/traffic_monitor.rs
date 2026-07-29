use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::utilities::mihomo_api::{MihomoClient, TrafficInfo};

const MAX_SAMPLES: usize = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSample {
    pub timestamp: u64,
    pub upload_bps: u64,
    pub download_bps: u64,
}

pub struct TrafficMonitor {
    samples: Arc<Mutex<VecDeque<TrafficSample>>>,
}

impl TrafficMonitor {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_SAMPLES))),
        }
    }

    pub fn spawn_collector(&self, mihomo_api_url: String) -> JoinHandle<()> {
        let samples = Arc::clone(&self.samples);
        tokio::spawn(async move {
            let client = MihomoClient::with_base_url(mihomo_api_url);
            let mut last: Option<TrafficInfo> = None;
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                interval.tick().await;
                match client.get_traffic().await {
                    Ok(current) => {
                        let (upload_bps, download_bps) = match &last {
                            Some(previous) => (
                                current.up.saturating_sub(previous.up),
                                current.down.saturating_sub(previous.down),
                            ),
                            None => (0, 0),
                        };
                        last = Some(current);
                        let sample = TrafficSample {
                            timestamp: now_secs(),
                            upload_bps,
                            download_bps,
                        };
                        let mut locked = samples.lock().await;
                        if locked.len() == MAX_SAMPLES {
                            locked.pop_front();
                        }
                        locked.push_back(sample);
                    }
                    Err(e) => {
                        warn!(error = %e, "traffic sample unavailable");
                    }
                }
            }
        })
    }

    pub async fn get_samples(&self, count: usize) -> Vec<TrafficSample> {
        let samples = self.samples.lock().await;
        let start = samples.len().saturating_sub(count);
        samples.iter().skip(start).cloned().collect()
    }
}

impl Default for TrafficMonitor {
    fn default() -> Self {
        Self::new()
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
