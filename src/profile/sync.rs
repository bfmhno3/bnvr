use rusqlite::{params, Connection};
use tracing::info;

use super::crud;

#[derive(Debug)]
pub struct SyncResult {
    pub name: String,
    pub bytes: usize,
    pub subscription_id: i64,
}

pub async fn fetch_yaml(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("bnvr")
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

    Ok(text)
}

pub async fn sync_one(
    conn: &Connection,
    name: &str,
) -> Result<SyncResult, Box<dyn std::error::Error>> {
    let profile = crud::get(conn, name)?;
    info!(name = %profile.name, url = %profile.url, "syncing profile");

    let content = fetch_yaml(&profile.url).await?;
    let bytes = content.len();

    // Validate YAML parses (warn but still store if it fails)
    if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
        tracing::warn!(name = %name, error = %e, "content is not valid YAML, storing raw text");
    }

    conn.execute(
        "INSERT INTO subscriptions (profile_id, content) VALUES (?1, ?2)",
        params![profile.id, content],
    )?;
    let sub_id = conn.last_insert_rowid();

    conn.execute(
        "UPDATE profiles SET raw_config = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![content, profile.id],
    )?;

    info!(name = %name, bytes, subscription_id = sub_id, "sync complete");

    Ok(SyncResult {
        name: name.to_string(),
        bytes,
        subscription_id: sub_id,
    })
}

pub async fn sync_all(conn: &Connection) -> Result<Vec<SyncResult>, Box<dyn std::error::Error>> {
    let profiles: Vec<crud::ProfileInfo> = crud::list(conn)?;
    let mut results = Vec::new();

    for profile in profiles {
        match sync_one(conn, &profile.name).await {
            Ok(r) => results.push(r),
            Err(e) => {
                tracing::error!(name = %profile.name, error = %e, "sync failed");
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::db;
    use rusqlite::Connection;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        conn
    }

    #[tokio::test]
    async fn test_sync_one_not_found() {
        let conn = test_conn();
        let result = sync_one(&conn, "nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_sync_all_empty() {
        let conn = test_conn();
        let results = sync_all(&conn).await.unwrap();
        assert!(results.is_empty());
    }
}
