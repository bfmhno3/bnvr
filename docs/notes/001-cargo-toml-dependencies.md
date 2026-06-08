---
id: 001
title: "Cargo.toml Dependencies"
tags: [rust, cargo, setup]
phase: 1
created: 2026-06-08
---

## What

`Cargo.toml` is the project manifest. The `[dependencies]` section declares external crates your project uses. Cargo downloads and compiles them automatically.

## Why

BNVR needs `clap` (CLI), `tokio` (async runtime), `serde` (serialization), `tracing` (logging), `rusqlite` (database), `reqwest` (HTTP). These are the foundation every phase builds on.

## How

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
rusqlite = { version = "0.40", features = ["bundled"] }
reqwest = { version = "0.13", features = ["json"], default-features = false }
```

**`features`** are compile-time flags. Not all code in a crate is enabled by default -- you opt in to what you need.

- `clap` with `"derive"` enables `#[derive(Parser)]` macros.
- `tokio` with `"full"` enables all features (`io`, `net`, `time`, `fs`, `sync`). You can be more selective later.
- `rusqlite` with `"bundled"` compiles SQLite from source -- no system dependency needed.
- `reqwest` uses rustls instead of OpenSSL. Avoids OpenSSL linking headaches on Windows.

## Gotchas

- `tokio` without `features = ["full"]` gives confusing "function not found" errors for things like `tokio::spawn`.
- `reqwest` default features include `default-tls` (OpenSSL). Setting `default-features = false` and adding `rustls-tls` avoids needing OpenSSL installed.

## Links

- [002-clap-derive-macros](./002-clap-derive-macros.md)
- [003-tracing-structured-logging](./003-tracing-structured-logging.md)
