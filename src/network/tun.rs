use std::error::Error;

use crate::daemon::ipc::{self, Request};

pub async fn setup_tun() -> Result<(), Box<dyn Error>> {
    let request = Request {
        id: 1,
        method: "tun_setup".to_string(),
        params: serde_json::Value::Null,
    };
    let response = ipc::send_request(&request).await?;
    if let Some(error) = response.error {
        return Err(error.into());
    }
    println!("TUN setup complete");
    Ok(())
}

pub async fn clear_tun() -> Result<(), Box<dyn Error>> {
    let request = Request {
        id: 1,
        method: "tun_clear".to_string(),
        params: serde_json::Value::Null,
    };
    let response = ipc::send_request(&request).await?;
    if let Some(error) = response.error {
        return Err(error.into());
    }
    println!("TUN cleared");
    Ok(())
}
