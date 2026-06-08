---
id: 008
title: "Rusqlite Database Setup"
tags: [rust, sqlite, database, rusqlite]
phase: 2
created: 2026-06-08
---

## What

`rusqlite` is a Rust binding for SQLite. `Connection::open(path)` creates or opens a database file. `execute_batch` runs multiple SQL statements at once.

## Why

BNVR stores profiles, subscriptions, bench results, and traffic stats in `~/.bnvr/bnvr.db`. SQLite is embedded -- no server needed.

## How

```rust
use rusqlite::Connection;

// For production: file-based
let conn = Connection::open(&db_path)?;
init_schema(&conn)?;

// For testing: in-memory
let conn = Connection::open_in_memory()?;
init_schema(&conn)?;
```

Separate `init_schema` from `open` so tests can use in-memory databases:

```rust
pub fn init_schema(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS profiles (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL UNIQUE,
            url         TEXT    NOT NULL,
            created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
        );
        -- ... more tables
    ")?;
    Ok(())
}
```

**`IF NOT EXISTS`** makes schema creation idempotent -- safe to call on every startup.

**`datetime('now')`** is SQLite's built-in UTC timestamp. Stored as text in ISO 8601 format.

**`open_in_memory()`** creates a temporary database in RAM. Disappears when the connection closes. Perfect for tests.

## Gotchas

- `PRAGMA` statements must be outside `CREATE TABLE` blocks. `execute_batch` handles this.
- `bundled` feature compiles SQLite from source. Adds ~1MB to binary but avoids system dependency.
- SQLite locks on concurrent writes. WAL mode allows concurrent reads during writes.
- `INTEGER PRIMARY KEY AUTOINCREMENT` prevents rowid reuse after deletion.
- **WAL mode doesn't work with in-memory databases.** `PRAGMA journal_mode=WAL` silently returns "memory" instead. Don't test WAL-specific behavior with `open_in_memory()`.
- Separate `init_schema(conn)` from `open()` so tests can pass an in-memory connection.

## Links

- [001-cargo-toml-dependencies](./001-cargo-toml-dependencies.md)
- [015-in-memory-sqlite-testing](./015-in-memory-sqlite-testing.md)
