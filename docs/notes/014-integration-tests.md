---
id: 014
title: "Integration Tests"
tags: [rust, testing, cargo]
phase: 2
created: 2026-06-08
---

## What

Integration tests live in `tests/` at the project root. Each `.rs` file is a separate test binary. They import the library crate (`use bnvr::...`) and test the public API from the outside.

## Why

Unit tests test internals. Integration tests test the public interface -- how modules work together. They catch issues that unit tests miss: wrong public signatures, wiring problems, real I/O behavior.

## How

```
tests/
  daemon_integration.rs
```

```rust
// tests/daemon_integration.rs
use bnvr::daemon::{db, ipc, paths};

#[test]
fn test_db_full_workflow() {
    let conn = Connection::open_in_memory().unwrap();
    db::init_schema(&conn).unwrap();
    // ... test the full API
}

#[tokio::test]
async fn test_ipc_status_roundtrip() {
    // ... start listener, connect client, verify response
}
```

**`#[tokio::test]`** for async tests. Same as `#[test]` but runs inside a tokio runtime.

**`cargo test`** runs all tests (unit + integration). **`cargo test --test daemon_integration`** runs only that integration test file.

**Each file in `tests/`** is compiled as a separate binary. They don't share state.

## Patterns

**Test helpers** at the top of the file:
```rust
async fn send_test_request(name: &str, req: &Request) -> Result<Response, String> {
    // ... connect, send, read response
}
```

**Unique names per test** to avoid conflicts:
```rust
let socket_name = "bnvr_test_status";   // unique per test
let socket_name = "bnvr_test_unknown";  // different name
```

**Cleanup temp data**:
```rust
let dir = std::env::temp_dir().join("bnvr_test");
let _ = std::fs::remove_dir_all(&dir);  // clean up before
std::fs::create_dir_all(&dir).unwrap();
// ... test ...
let _ = std::fs::remove_dir_all(&dir);  // clean up after
```

## Gotchas

- Integration tests can only `use` the library crate (`bnvr::...`), not private modules. Everything they test must be `pub`.
- Without `src/lib.rs`, `use bnvr::...` won't compile. See [012-binary-vs-library-crate](./012-binary-vs-library-crate.md).
- `#[tokio::test]` requires `tokio` in `[dev-dependencies]` or `[dependencies]`. We have it in `[dependencies]`.
- Tests run in parallel by default. Use unique socket names / temp dirs to avoid conflicts.
- `cargo test -- --test-threads=1` runs tests sequentially if needed.

## Links

- [012-binary-vs-library-crate](./012-binary-vs-library-crate.md)
- [013-unit-testing-with-cfg-test](./013-unit-testing-with-cfg-test.md)
- [015-in-memory-sqlite-testing](./015-in-memory-sqlite-testing.md)
