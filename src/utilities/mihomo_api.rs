use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficInfo {
    #[serde(default)]
    pub up: u64,
    #[serde(default)]
    pub down: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    #[serde(default)]
    pub delay: Option<u32>,
}

pub struct MihomoClient {
    base_url: String,
    client: Client,
}

impl MihomoClient {
    pub fn new(port: u16) -> Self {
        Self::with_base_url(format!("http://127.0.0.1:{port}"))
    }

    pub fn with_base_url(base_url: String) -> Self {
        let client = Client::builder()
            .user_agent("bnvr")
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { base_url, client }
    }

    pub async fn get_traffic(
        &self,
    ) -> Result<TrafficInfo, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/traffic", self.base_url);
        let traffic = self
            .client
            .get(url)
            .send()
            .await?
            .json::<TrafficInfo>()
            .await?;
        Ok(traffic)
    }

    pub async fn get_proxies(
        &self,
    ) -> Result<Vec<ProxyInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/proxies", self.base_url);
        let response = self
            .client
            .get(url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let Some(proxies) = response
            .get("proxies")
            .and_then(serde_json::Value::as_object)
        else {
            return Ok(Vec::new());
        };

        let mut result = Vec::new();
        for (name, value) in proxies {
            if matches!(name.as_str(), "GLOBAL" | "DIRECT" | "REJECT") {
                continue;
            }
            let proxy_type = value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let delay = value
                .get("history")
                .and_then(serde_json::Value::as_array)
                .and_then(|history| history.last())
                .and_then(|entry| entry.get("delay"))
                .and_then(serde_json::Value::as_u64)
                .and_then(|delay| u32::try_from(delay).ok());
            result.push(ProxyInfo {
                name: name.clone(),
                proxy_type,
                delay,
            });
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    pub async fn get_current_proxy(
        &self,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/proxies", self.base_url);
        let response = self
            .client
            .get(url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        Ok(response
            .get("proxies")
            .and_then(|value| value.get("GLOBAL"))
            .and_then(|value| value.get("now"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string))
    }

    pub async fn test_delay(
        &self,
        proxy_name: &str,
    ) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        let encoded = percent_encode(proxy_name);
        let url = format!(
            "{}/proxies/{encoded}/delay?timeout=5000&url=https://www.gstatic.com/generate_204",
            self.base_url
        );
        let response = self
            .client
            .get(url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let delay = response
            .get("delay")
            .and_then(serde_json::Value::as_u64)
            .ok_or("mihomo delay response missing delay")?;
        u32::try_from(delay).map_err(Into::into)
    }
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}
