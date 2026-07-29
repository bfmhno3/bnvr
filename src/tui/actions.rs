use crate::daemon::ipc::{self, Request};
use crate::utilities::mihomo_api::MihomoClient;

pub async fn switch_node(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let req = Request {
        id: 1,
        method: "switch_node".to_string(),
        params: serde_json::json!({ "node_name": name }),
    };
    let response = ipc::send_request(&req).await?;
    if let Some(error) = response.error {
        return Err(error.into());
    }
    Ok(())
}

pub async fn test_node_delay(name: &str) -> Result<u32, Box<dyn std::error::Error>> {
    MihomoClient::new(9090)
        .test_delay(name)
        .await
        .map_err(|e| e.to_string().into())
}
