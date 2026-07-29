use std::sync::Arc;

use tokio::sync::Mutex;

use super::core::KernelManager;
use super::tun_state::TunState;

pub struct DaemonState {
    pub kernel: Arc<KernelManager>,
    pub tun: Arc<Mutex<TunState>>,
}

impl DaemonState {
    pub fn new(kernel: Arc<KernelManager>) -> Self {
        Self {
            kernel,
            tun: Arc::new(Mutex::new(TunState::new())),
        }
    }
}
