use std::error::Error;
use std::net::IpAddr;

use ipnetwork::IpNetwork;

use crate::daemon::ipc::{self, Request};

pub async fn add_bypass_route(target: &str) -> Result<(), Box<dyn Error>> {
    let target = normalize_target(target)?;
    let request = Request {
        id: 1,
        method: "add_bypass".to_string(),
        params: serde_json::json!({ "target": target }),
    };
    let response = ipc::send_request(&request).await?;
    if let Some(error) = response.error {
        return Err(error.into());
    }
    println!("bypass route added: {target}");
    Ok(())
}

pub fn normalize_target(target: &str) -> Result<String, Box<dyn Error>> {
    if let Ok(network) = target.parse::<IpNetwork>() {
        return Ok(network.to_string());
    }

    let ip: IpAddr = target.parse()?;
    Ok(match ip {
        IpAddr::V4(addr) => format!("{addr}/32"),
        IpAddr::V6(addr) => format!("{addr}/128"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_bypass_target_accepts_ip_and_cidr() {
        assert_eq!(normalize_target("10.0.0.1").unwrap(), "10.0.0.1/32");
        assert_eq!(
            normalize_target("192.168.1.0/24").unwrap(),
            "192.168.1.0/24"
        );
    }

    #[test]
    fn test_normalize_bypass_target_rejects_invalid() {
        assert!(normalize_target("not-an-ip").is_err());
    }
}
