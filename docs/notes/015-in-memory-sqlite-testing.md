---
id: 015
title: "In-Memory SQLite for Testing"
tags: [rust, sqlite, testing]
phase: 2
created: 2026-06-08
---

## What

`Connection::open_in_memory()` creates a SQLite database that lives only in RAM. It disappears when the connection closes. No files, no cleanup.

## Why

Tests should not modify real databases. In-memory databases are fast (no disk I/O) and isolated (each test gets a fresh DB).

## How

```rust
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
    }
}
```

The pattern: `open_in_memory()` + `init_schema(&conn)` + test. Each test gets a clean database.

**Separate `init_schema(conn)` from `open()`** in production code:
```rust
// db.rs
pub fn open() -> Result<Connection, ...> {
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;
    Ok(conn)
}

pub fn init_schema(conn: &Connection) -> Result<(), ...> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS ...")?;
    Ok(())
}
```

Tests call `init_schema` directly with an in-memory connection.

## Gotchas

- **WAL mode doesn't work in-memory.** `PRAGMA journal_mode=WAL` silently returns `"memory"`. Don't test WAL-specific behavior with in-memory DB.
- **Foreign keys are off by default** in SQLite. `PRAGMA foreign_keys=ON` must be set per connection. `init_schema` handles this.
- **`open_in_memory()` is not `:memory:`.** The `rusqlite` API uses `Connection::open_in_memory()`, not `Connection::open(":memory:")`. Both work, but the method is more idiomatic.
- **Each test gets its own connection.** Don't share a single in-memory connection across tests -- it won't be isolated.

## Links

- [008-rusqlite-database-setup](./008-rusqlite-database-setup.md)
- [013-unit-testing-with-cfg-test](./013-unit-testing-with-cfg-test.md)
