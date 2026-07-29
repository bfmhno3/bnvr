use std::error::Error;

use crate::network::device::TunDevice;

pub struct TunState {
    device: Option<TunDevice>,
    enabled: bool,
}

impl TunState {
    pub fn new() -> Self {
        Self {
            device: None,
            enabled: false,
        }
    }

    pub fn setup(&mut self) -> Result<(), Box<dyn Error>> {
        if self.enabled {
            return Err("TUN is already enabled".into());
        }

        let device = TunDevice::create_default()?;
        self.device = Some(device);
        self.enabled = true;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(device) = self.device.take() {
            device.destroy()?;
        }
        self.enabled = false;
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn device_name(&self) -> Option<&str> {
        if self.enabled {
            self.device.as_ref().map(TunDevice::name)
        } else {
            None
        }
    }
}

impl Default for TunState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_disabled() {
        let state = TunState::new();
        assert!(!state.is_enabled());
        assert!(state.device_name().is_none());
    }

    #[test]
    fn test_clear_disabled_is_idempotent() {
        let mut state = TunState::new();
        state.clear().unwrap();
        assert!(!state.is_enabled());
    }
}
