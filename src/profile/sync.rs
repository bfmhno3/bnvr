use std::error::Error;
use std::path::PathBuf;

use tracing::info;

use super::crud::{self, ProfileKind};
use crate::paths;

const DEFAULT_USER_AGENT: &str = concat!("clash-verge/v", env!("CARGO_PKG_VERSION"));

#[derive(Debug)]
pub struct SyncResult {
    pub name: String,
    pub bytes: usize,
    pub path: PathBuf,
}

pub struct SyncFailure {
    pub name: String,
    pub error: String,
}

pub struct SyncAllResult {
    pub synced: Vec<SyncResult>,
    pub failed: Vec<SyncFailure>,
}

pub async fn fetch_yaml(url: &str, user_agent: Option<&str>) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .user_agent(user_agent.unwrap_or(DEFAULT_USER_AGENT))
        .build()?;

    let resp = client.get(url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status} from {url}").into());
    }

    let text = resp.text().await?;
    if text.is_empty() {
        return Err("empty response from subscription URL".into());
    }

    Ok(text.trim_start_matches('\u{feff}').to_string())
}

pub fn validate_config(content: &str) -> Result<(), Box<dyn Error>> {
    let value: serde_yaml::Value = serde_yaml::from_str(content)?;
    let mapping = value
        .as_mapping()
        .ok_or("invalid config: expected a YAML mapping")?;
    if !mapping.contains_key("proxies") && !mapping.contains_key("proxy-providers") {
        return Err("invalid config: missing `proxies` and `proxy-providers`".into());
    }
    Ok(())
}

pub async fn sync_one(name: &str) -> Result<SyncResult, Box<dyn Error>> {
    let profile = crud::get(name)?;
    let url = profile
        .meta
        .url
        .as_deref()
        .ok_or_else(|| format!("profile {name} has no url"))?;
    info!(name = %profile.name, url = %url, "syncing profile");

    let content = fetch_yaml(url, profile.meta.user_agent.as_deref()).await?;
    validate_config(&content)?;
    let bytes = content.len();
    let path = paths::profile_raw_file(name);
    crud::write_atomic(&path, &content)?;

    let mut meta = profile.meta;
    meta.updated_at = Some(crud::now_secs());
    crud::write_meta(name, &meta)?;
    crud::refresh_active_config(name).await?;

    info!(name = %name, bytes, "sync complete");

    Ok(SyncResult {
        name: name.to_string(),
        bytes,
        path,
    })
}

pub async fn sync_all() -> Result<SyncAllResult, Box<dyn Error>> {
    let profiles = crud::list()?;
    let mut synced = Vec::new();
    let mut failed = Vec::new();

    for profile in profiles {
        if profile.meta.kind == ProfileKind::Merge {
            continue;
        }
        match sync_one(&profile.name).await {
            Ok(r) => synced.push(r),
            Err(e) => {
                tracing::error!(name = %profile.name, error = %e, "sync failed");
                failed.push(SyncFailure {
                    name: profile.name,
                    error: e.to_string(),
                });
            }
        }
    }

    Ok(SyncAllResult { synced, failed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn setup(test_name: &str) -> (PathBuf, std::sync::MutexGuard<'static, ()>) {
        test_env::setup_profile(&format!("sync-{test_name}"))
    }

    fn cleanup(tmp: &PathBuf) {
        test_env::cleanup(tmp);
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_sync_one_not_found() {
        let (tmp, _guard) = setup("one-not-found");
        let result = sync_one("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
        cleanup(&tmp);
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_sync_all_empty() {
        let (tmp, _guard) = setup("all-empty");
        let results = sync_all().await.unwrap();
        assert!(results.synced.is_empty());
        assert!(results.failed.is_empty());
        cleanup(&tmp);
    }
    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_sync_all_returns_successes_and_failures() {
        let (tmp, _guard) = setup("success-failure");
        let (good_url, _) =
            start_test_http_server("HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nproxies: []");
        let (bad_url, _) = start_test_http_server(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n",
        );
        crud::add("bad", &bad_url, None).unwrap();
        crud::add("good", &good_url, None).unwrap();

        let result = sync_all().await.unwrap();
        assert_eq!(result.synced.len(), 1);
        assert_eq!(result.synced[0].name, "good");
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].name, "bad");
        assert!(result.failed[0].error.contains("HTTP 500"));
        cleanup(&tmp);
    }

    fn start_test_http_server(response: &'static str) -> (String, Arc<Mutex<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let request = Arc::new(Mutex::new(String::new()));
        let captured = Arc::clone(&request);
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 1024];
            let n = stream.read(&mut buffer).unwrap_or(0);
            *captured.lock().unwrap() = String::from_utf8_lossy(&buffer[..n]).to_string();
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{addr}/sub.yaml"), request)
    }
}
