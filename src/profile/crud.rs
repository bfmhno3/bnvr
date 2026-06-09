use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct ProfileInfo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub raw_config: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn add(conn: &Connection, name: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute(
        "INSERT INTO profiles (name, url) VALUES (?1, ?2)",
        params![name, url],
    )?;
    Ok(())
}

pub fn del(conn: &Connection, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rows = conn.execute("DELETE FROM profiles WHERE name = ?1", params![name])?;
    if rows == 0 {
        return Err(format!("profile not found: {name}").into());
    }
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<ProfileInfo>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, raw_config, created_at, updated_at FROM profiles ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ProfileInfo {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            raw_config: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get(conn: &Connection, name: &str) -> Result<ProfileInfo, Box<dyn std::error::Error>> {
    conn.query_row(
        "SELECT id, name, url, raw_config, created_at, updated_at FROM profiles WHERE name = ?1",
        params![name],
        |row| {
            Ok(ProfileInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                raw_config: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => format!("profile not found: {name}").into(),
        other => other.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::db;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn test_add_and_get() {
        let conn = test_conn();
        add(&conn, "test", "http://example.com").unwrap();
        let p = get(&conn, "test").unwrap();
        assert_eq!(p.name, "test");
        assert_eq!(p.url, "http://example.com");
    }

    #[test]
    fn test_add_duplicate_name_fails() {
        let conn = test_conn();
        add(&conn, "dup", "http://a.com").unwrap();
        let result = add(&conn, "dup", "http://b.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_del_existing() {
        let conn = test_conn();
        add(&conn, "test", "http://example.com").unwrap();
        del(&conn, "test").unwrap();
        let result = get(&conn, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_del_not_found() {
        let conn = test_conn();
        let result = del(&conn, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_empty() {
        let conn = test_conn();
        let profiles = list(&conn).unwrap();
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_list_multiple() {
        let conn = test_conn();
        add(&conn, "b", "http://b.com").unwrap();
        add(&conn, "a", "http://a.com").unwrap();
        let profiles = list(&conn).unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "a");
        assert_eq!(profiles[1].name, "b");
    }

    #[test]
    fn test_get_not_found() {
        let conn = test_conn();
        let result = get(&conn, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_del_cascades_subscriptions() {
        let conn = test_conn();
        add(&conn, "test", "http://example.com").unwrap();
        let p = get(&conn, "test").unwrap();
        conn.execute(
            "INSERT INTO subscriptions (profile_id, content) VALUES (?1, 'content')",
            params![p.id],
        )
        .unwrap();
        del(&conn, "test").unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
