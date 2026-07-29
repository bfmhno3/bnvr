use std::error::Error;
use std::net::Ipv4Addr;

use crate::network::privilege;

pub const TUN_DEVICE_NAME: &str = if cfg!(target_os = "windows") {
    "bnvr0"
} else {
    "tun-bnvr0"
};
pub const TUN_MTU: u16 = 9000;
pub const TUN_IPV4: &str = "198.18.0.1";
pub const TUN_IPV4_NETMASK: &str = "255.255.0.0";

pub struct TunDevice {
    name: String,
    mtu: u16,
    device: tun2::Device,
}

impl TunDevice {
    pub fn create(
        name: &str,
        mtu: u16,
        ipv4: Ipv4Addr,
        ipv4_netmask: Ipv4Addr,
    ) -> Result<Self, Box<dyn Error>> {
        validate_mtu(mtu)?;
        privilege::check_privileges()?;

        let mut config = tun2::Configuration::default();
        config
            .tun_name(name)
            .address(ipv4)
            .netmask(ipv4_netmask)
            .mtu(mtu)
            .up();

        let device = tun2::create(&config).map_err(map_create_error)?;

        Ok(Self {
            name: name.to_string(),
            mtu,
            device,
        })
    }

    pub fn create_default() -> Result<Self, Box<dyn Error>> {
        Self::create(
            TUN_DEVICE_NAME,
            TUN_MTU,
            TUN_IPV4.parse()?,
            TUN_IPV4_NETMASK.parse()?,
        )
    }

    pub fn destroy(self) -> Result<(), Box<dyn Error>> {
        drop(self.device);
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mtu(&self) -> u16 {
        self.mtu
    }
}

fn validate_mtu(mtu: u16) -> Result<(), Box<dyn Error>> {
    if !(1280..=65535).contains(&mtu) {
        return Err(format!("invalid TUN MTU {mtu}: expected 1280..=65535").into());
    }
    Ok(())
}

fn map_create_error(error: tun2::Error) -> Box<dyn Error> {
    let message = error.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("exist") || lower.contains("in use") || lower.contains("busy") {
        return "device already exists, run 'bnvr network tun clear' first".into();
    }
    if lower.contains("permission") || lower.contains("access") || lower.contains("privilege") {
        if cfg!(target_os = "windows") {
            return "requires administrator (Windows) or CAP_NET_ADMIN/root (Linux)".into();
        }
        return "requires administrator (Windows) or CAP_NET_ADMIN/root (Linux)".into();
    }
    message.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_mtu_bounds() {
        assert!(validate_mtu(1280).is_ok());
        assert!(validate_mtu(u16::MAX).is_ok());
        assert!(validate_mtu(1279).is_err());
    }
}
