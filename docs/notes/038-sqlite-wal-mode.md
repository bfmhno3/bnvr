---
id: 038
title: "SQLite WAL Mode and Concurrent Access"
tags: [rust, sqlite, database, concurrency]
phase: 5
created: 2026-06-09
---

## What

WAL (Write-Ahead Logging) is a SQLite journal mode that allows concurrent reads while a write is in progress. It writes changes to a separate `-wal` file instead of modifying the main database file directly.

## Why

BNVR's daemon and CLI may access `~/.bnvr/bnvr.db` simultaneously. WAL mode prevents the CLI from blocking on daemon writes, and vice versa.

## How

Enabled in `db::init_schema()`:

```rust
conn.execute_batch("
    PRAGMA journal_mode=WAL;
    PRAGMA foreign_keys=ON;
")?;
```

**Key properties**:
- Multiple readers can proceed concurrently with one writer
- Writers do not block readers (unlike the default rollback journal)
- `PRAGMA foreign_keys=ON` must be set on every connection (not persisted)
- WAL mode is persisted in the database file after first use

**Gotcha**: WAL mode requires `PRAGMA journal_mode=WAL` only on the first connection that modifies the DB. Subsequent connections inherit it. But `PRAGMA foreign_keys=ON` must be set on every connection.

## Links

- [008-rusqlite-database-setup](./008-rusqlite-database-setup.md) - initial schema setup
- [034-rusqlite-insert-delete](./034-rusqlite-insert-delete.md) - INSERT/DELETE operations
