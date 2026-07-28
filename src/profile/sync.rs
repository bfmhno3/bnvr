use rusqlite::{Connection, params};
use tracing::info;

use super::crud;

#[derive(Debug)]
pub struct SyncResult {
    pub name: String,
    pub bytes: usize,
    pub subscription_id: i64,
}

pub struct SyncFailure {
    pub name: String,
    pub error: String,
}

pub struct SyncAllResult {
    pub synced: Vec<SyncResult>,
    pub failed: Vec<SyncFailure>,
}

pub async fn fetch_yaml(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder().user_agent("bnvr").build()?;

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

    if let Err(e) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
        tracing::warn!(name = %name, error = %e, "content is not valid YAML, storing raw text");
    }

    let sub_id = store_sync(conn, profile.id, &content)?;

    info!(name = %name, bytes, subscription_id = sub_id, "sync complete");

    Ok(SyncResult {
        name: name.to_string(),
        bytes,
        subscription_id: sub_id,
    })
}

fn store_sync(
    conn: &Connection,
    profile_id: i64,
    content: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO subscriptions (profile_id, content) VALUES (?1, ?2)",
        params![profile_id, content],
    )?;
    let sub_id = tx.last_insert_rowid();
    tx.execute(
        "UPDATE profiles SET raw_config = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![content, profile_id],
    )?;
    tx.commit()?;
    Ok(sub_id)
}

pub async fn sync_all(conn: &Connection) -> Result<SyncAllResult, Box<dyn std::error::Error>> {
    let profiles: Vec<crud::ProfileInfo> = crud::list(conn)?;
    let mut synced = Vec::new();
    let mut failed = Vec::new();

    for profile in profiles {
        match sync_one(conn, &profile.name).await {
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
    use crate::daemon::db;
    use rusqlite::Connection;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::thread;

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
        assert!(results.synced.is_empty());
        assert!(results.failed.is_empty());
    }

    #[test]
    fn test_store_sync_rolls_back_when_profile_update_fails() {
        let conn = test_conn();
        crud::add(&conn, "test", "http://example.test/sub.yaml").unwrap();
        let profile = crud::get(&conn, "test").unwrap();
        conn.execute(
            "CREATE TRIGGER abort_profile_update BEFORE UPDATE ON profiles BEGIN SELECT RAISE(ABORT, 'abort update'); END",
            [],
        )
        .unwrap();

        let result = store_sync(&conn, profile.id, "proxies: []");
        assert!(result.is_err());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
            .unwrap();
        let raw_config: Option<String> = conn
            .query_row(
                "SELECT raw_config FROM profiles WHERE id = ?1",
                [profile.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        assert!(raw_config.is_none());
    }

    #[tokio::test]
    async fn test_sync_all_returns_successes_and_failures() {
        let conn = test_conn();
        let good_url =
            start_test_http_server("HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\nproxies: []");
        let bad_url = start_test_http_server(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n",
        );
        crud::add(&conn, "bad", &bad_url).unwrap();
        crud::add(&conn, "good", &good_url).unwrap();

        let result = sync_all(&conn).await.unwrap();
        assert_eq!(result.synced.len(), 1);
        assert_eq!(result.synced[0].name, "good");
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].name, "bad");
        assert!(result.failed[0].error.contains("HTTP 500"));
    }

    fn start_test_http_server(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer);
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}/sub.yaml")
    }
}
