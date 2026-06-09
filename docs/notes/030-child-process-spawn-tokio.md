---
id: 030
title: "Spawning Child Processes with tokio::process"
tags: [rust, tokio, process, async]
phase: 4
created: 2026-06-09
---

## What

`tokio::process::Command` is the async equivalent of `std::process::Command`. It spawns child processes that integrate with tokio's event loop, allowing non-blocking waits and timeouts.

## Why

BNVR needs to spawn Mihomo as a child process from the daemon. Using `std::process::Command` would block the async runtime. `tokio::process::Command` lets us spawn, monitor, and kill the child without blocking other async tasks like IPC handling.

## How

```rust
use tokio::process::Command;

// Spawn a child process
let child = Command::new("/path/to/mihomo")
    .arg("-d")
    .arg("/path/to/config")
    .spawn()?;

// Get PID
let pid = child.id().expect("failed to get PID");

// Wait for exit (non-blocking)
let status = child.wait().await?;

// Kill the child
child.kill().await?;
```

Key differences from `std::process::Command`:
- `.spawn()` returns a `tokio::process::Child` (not `std::process::Child`)
- `.wait()` is `async` -- yields to the runtime while waiting
- `.kill()` is `async` -- sends SIGKILL / TerminateProcess
- `.id()` returns `Option<u32>` (None if already exited)

Gotcha: `child.wait()` consumes the exit status. Call it once, store the result. If you need to both wait and kill, use `tokio::select!`.

## Links

- [031-process-lifecycle-monitoring](./031-process-lifecycle-monitoring.md)
- [002-tokio-async-runtime](./002-tokio-async-runtime.md)
