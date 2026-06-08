---
id: 009
title: "Tokio Shutdown Signal"
tags: [rust, tokio, async, signal]
phase: 2
created: 2026-06-08
---

## What

`tokio::signal::ctrl_c()` returns a future that resolves when the process receives SIGINT (ctrl-c) or SIGTERM (on some platforms).

## Why

The daemon must shut down gracefully: close the IPC listener, remove the PID file, flush logs. A shutdown signal triggers this cleanup.

## How

```rust
// Start background tasks
let ipc_handle = tokio::spawn(async { ... });

// Block until signal
tokio::signal::ctrl_c().await?;
info!("shutting down");

// Clean up
ipc_handle.abort();
fs::remove_file(&pid_path)?;
```

`ctrl_c()` is a one-shot future. After it resolves, calling it again creates a new listener.

`abort()` cancels a spawned task. It does not wait for the task to finish -- the task's future is dropped immediately.

## Gotchas

- `ctrl_c()` requires the `signal` feature in tokio. `features = ["full"]` includes it.
- On Windows, ctrl-c handling has caveats for non-console processes. For `daemon stop`, we use `TerminateProcess` instead.
- If the daemon is spawned from a shell, ctrl-c in the shell sends SIGINT to the whole process group. The daemon and the shell both receive it.

## Links

- [005-pid-file-management](./005-pid-file-management.md)
- [011-interprocess-ipc](./011-interprocess-ipc.md)
