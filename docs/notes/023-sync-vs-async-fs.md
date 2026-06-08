---
id: 023
title: "Synchronous vs Asynchronous File I/O"
tags: [rust, tokio, async, filesystem]
phase: 4
created: 2026-06-08
---

## What

Choosing between `std::fs` (blocking) and `tokio::fs` (async) for file operations.

## Why

BNVR's kernel management commands (`list`, `install`, `use`, `status`) do file I/O. The question is whether to use blocking `std::fs` or async `tokio::fs`.

## How

**Use `std::fs` when:**
- Running from a CLI command (not inside a tokio runtime task)
- File operations are small and fast (read a few KB, write a version string)
- The function is called synchronously from `main()`

**Use `tokio::fs` when:**
- Running inside a tokio task (e.g., daemon background work)
- Doing large I/O that could block the runtime (downloading multi-MB files)
- Need to `await` alongside other async operations

```rust
// CLI command -- synchronous is fine
fn list_installed() -> Vec<KernelInfo> {
    let entries = std::fs::read_dir(&kernels_dir)?;  // blocking, but <1ms
    // ...
}

// Daemon task -- use async
async fn monitor_kernel() {
    let output = tokio::fs::read_to_string(pid_file).await?;
    // ...
}
```

Key points:
- `tokio::fs` spawns blocking ops onto a thread pool internally -- not truly non-blocking
- For small files (<1MB), the overhead of `tokio::fs` is worse than just using `std::fs`
- BNVR's kernel CLI commands use `std::fs` (simpler, no runtime dependency)

## Links

- [009-tokio-shutdown-signal](./009-tokio-shutdown-signal.md)
- [005-pid-file-management](./005-pid-file-management.md)
