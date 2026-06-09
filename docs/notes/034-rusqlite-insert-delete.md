---
id: 034
title: "rusqlite INSERT/DELETE with Foreign Keys"
tags: [rust, sqlite, database, rusqlite]
phase: 5
created: 2026-06-09
---

## What

rusqlite provides parameterized SQL execution for SQLite. INSERT adds rows, DELETE removes them. With `PRAGMA foreign_keys=ON` and `ON DELETE CASCADE`, deleting a parent row automatically deletes child rows.

## Why

Profile CRUD operations need to insert/delete from the `profiles` table. When a profile is deleted, its `subscriptions` rows should be cleaned up automatically via CASCADE.

## How

Parameterized INSERT:

```rust
conn.execute(
    "INSERT INTO profiles (name, url) VALUES (?1, ?2)",
    params![name, url],
)?;
```

DELETE with row count check:

```rust
let rows = conn.execute("DELETE FROM profiles WHERE name = ?1", params![name])?;
if rows == 0 {
    return Err("profile not found".into());
}
```

Handling UNIQUE constraint violations:

```rust
match conn.execute("INSERT INTO profiles ...", params![...]) {
    Ok(_) => {}
    Err(rusqlite::Error::SqliteFailure(e, _))
        if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
    {
        return Err("profile name already exists".into());
    }
    Err(e) => return Err(e.into()),
}
```

CASCADE requires `PRAGMA foreign_keys=ON` on every connection (it is not persisted). This is already set in `db::init_schema()`.

## Links

- [008-rusqlite-database-setup](./008-rusqlite-database-setup.md) - schema setup
- [015-in-memory-sqlite-testing](./015-in-memory-sqlite-testing.md) - testing with in-memory DB
