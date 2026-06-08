---
id: 012
title: "Binary vs Library Crate"
tags: [rust, cargo, project-structure, testing]
phase: 2
created: 2026-06-08
---

## What

A Rust project can have both `src/main.rs` (binary crate) and `src/lib.rs` (library crate). The binary is the executable. The library is importable by other code -- including integration tests.

## Why

Integration tests in `tests/` live outside the crate. They can only import the library crate (`use bnvr::daemon`), not the binary's internal modules. Without `src/lib.rs`, integration tests can't access your code.

## How

```
src/
  main.rs    <- binary crate (has `fn main`)
  lib.rs     <- library crate (re-exports modules)
  daemon/    <- actual code
  cli.rs     <- binary-only (CLI parsing)
```

**`src/lib.rs`**:
```rust
pub mod daemon;
```

**`src/main.rs`**:
```rust
mod cli;           // private to binary
use bnvr::daemon;  // import from library
```

`cli.rs` stays in `main.rs` only -- it's the binary's entry point, not reusable code. The `daemon` module lives in `lib.rs` so both the binary and integration tests can use it.

## Gotchas

- If you only have `src/main.rs`, `use bnvr::anything` in tests won't compile. You need `src/lib.rs`.
- Both `main.rs` and `lib.rs` can declare `mod daemon;` -- they'll share the same source files. But prefer putting shared code in `lib.rs` and having `main.rs` use `bnvr::daemon`.
- `pub mod daemon;` in `lib.rs` makes it visible to integration tests. `mod daemon;` (no `pub`) would keep it private.
- The binary crate name is the package name from `Cargo.toml`. `use bnvr::...` in tests references the library crate.

## Links

- [014-integration-tests](./014-integration-tests.md)
- [001-cargo-toml-dependencies](./001-cargo-toml-dependencies.md)
