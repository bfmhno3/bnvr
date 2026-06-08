---
id: 003
title: "Tracing Structured Logging"
tags: [rust, tracing, logging, async]
phase: 1
created: 2026-06-08
---

## What

`tracing` is a structured logging and diagnostics framework. Unlike `println!`, it supports log levels (trace/debug/info/warn/error), structured key-value fields, and async-aware span tracking.

## Why

BNVR is a daemon + TUI. You need log levels (silence debug output in production), structured fields (log PID, socket path, etc. as searchable data), and async context (which task produced which log line). `println!` can't do any of that.

## How

```rust
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("bnvr starting");
    info!(port = 7890, "daemon listening");
}
```

**`EnvFilter`** reads the `RUST_LOG` environment variable to control log levels at runtime:

```bash
RUST_LOG=debug cargo run -- daemon start    # show debug+info+warn+error
RUST_LOG=bnvr=trace cargo run -- daemon start  # trace only for bnvr crate
```

Without `RUST_LOG`, defaults to `info`.

## Structured fields

```rust
info!(pid = 12345, version = "1.5.0", "kernel started");
warn!(timeout_ms = 3000, "python script killed");
```

These become key-value pairs in the output. Much easier to grep/filter than interpolated strings.

## Gotchas

- `tracing_subscriber::fmt()` must be initialized once. Calling `.init()` twice panics.
- `EnvFilter::new("info")` is the fallback when `RUST_LOG` is not set. If you omit it, you get no output at the `info` level.
- `tracing` and `log` are different crates. Some older libraries use `log`. `tracing-subscriber` with the `env-filter` feature can capture `log` records too, but you need `tracing-log` bridge (usually pulled in automatically).

## Links

- [001-cargo-toml-dependencies](./001-cargo-toml-dependencies.md)
