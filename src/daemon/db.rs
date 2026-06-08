use rusqlite::Connection;

use super::paths;

pub fn open() -> Result<Connection, Box<dyn std::error::Error>> {
    let db_path = paths::db_file();
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;
    Ok(conn)
}

pub fn init_schema(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS profiles (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL UNIQUE,
            url         TEXT    NOT NULL,
            raw_config  TEXT,
            created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS subscriptions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            profile_id  INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
            content     TEXT    NOT NULL,
            fetched_at  TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS audit_log (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            action      TEXT    NOT NULL,
            detail      TEXT,
            created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS bench_results (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            group_name      TEXT,
            node            TEXT    NOT NULL,
            connect_ms      REAL,
            tls_ms          REAL,
            jitter_ms       REAL,
            created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS traffic_stats (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            domain      TEXT    NOT NULL,
            bytes_up    INTEGER NOT NULL DEFAULT 0,
            bytes_down  INTEGER NOT NULL DEFAULT 0,
            recorded_at TEXT    NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_schema_creates_all_tables() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"profiles".to_string()));
        assert!(tables.contains(&"subscriptions".to_string()));
        assert!(tables.contains(&"audit_log".to_string()));
        assert!(tables.contains(&"bench_results".to_string()));
        assert!(tables.contains(&"traffic_stats".to_string()));
    }

    #[test]
    fn test_schema_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        init_schema(&conn).unwrap(); // second call should not fail
    }

    #[test]
    fn test_profiles_table_columns() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        conn.execute("INSERT INTO profiles (name, url) VALUES ('test', 'http://example.com')", [])
            .unwrap();

        let (name, url): (String, String) = conn
            .query_row("SELECT name, url FROM profiles WHERE name='test'", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();

        assert_eq!(name, "test");
        assert_eq!(url, "http://example.com");
    }

    #[test]
    fn test_profiles_unique_name_constraint() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        conn.execute("INSERT INTO profiles (name, url) VALUES ('dup', 'http://a.com')", [])
            .unwrap();
        let result = conn.execute(
            "INSERT INTO profiles (name, url) VALUES ('dup', 'http://b.com')",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_subscriptions_foreign_key() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        conn.execute("INSERT INTO profiles (name, url) VALUES ('p1', 'http://a.com')", [])
            .unwrap();

        conn.execute(
            "INSERT INTO subscriptions (profile_id, content) VALUES (1, 'content')",
            [],
        )
        .unwrap();

        let content: String = conn
            .query_row("SELECT content FROM subscriptions WHERE profile_id=1", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(content, "content");
    }

    #[test]
    fn test_subscriptions_cascade_delete() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        conn.execute("INSERT INTO profiles (name, url) VALUES ('p1', 'http://a.com')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO subscriptions (profile_id, content) VALUES (1, 'content')",
            [],
        )
        .unwrap();

        conn.execute("DELETE FROM profiles WHERE id=1", []).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_audit_log_insert() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO audit_log (action, detail) VALUES ('test_action', 'test_detail')",
            [],
        )
        .unwrap();

        let action: String = conn
            .query_row("SELECT action FROM audit_log WHERE id=1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(action, "test_action");
    }

    #[test]
    fn test_bench_results_insert() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO bench_results (group_name, node, connect_ms, tls_ms, jitter_ms) VALUES ('g1', 'node1', 10.5, 20.3, 1.2)",
            [],
        )
        .unwrap();

        let node: String = conn
            .query_row("SELECT node FROM bench_results WHERE id=1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(node, "node1");
    }

    #[test]
    fn test_traffic_stats_defaults() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        conn.execute(
            "INSERT INTO traffic_stats (domain) VALUES ('example.com')",
            [],
        )
        .unwrap();

        let (bytes_up, bytes_down): (i64, i64) = conn
            .query_row(
                "SELECT bytes_up, bytes_down FROM traffic_stats WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(bytes_up, 0);
        assert_eq!(bytes_down, 0);
    }

}
