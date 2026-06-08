---
id: 005
title: "PID File Management"
tags: [rust, daemon, filesystem, process]
phase: 2
created: 2026-06-08
---

## What

A PID file stores a process ID as plain text. Daemons write it on start and remove it on stop. Other processes read it to find the daemon.

## Why

`bnvr daemon start` needs to prevent duplicate instances. `bnvr daemon stop` needs to know which process to kill. The PID file at `~/.bnvr/bnvr.pid` is the coordination point.

## How

```rust
use std::fs;

// Write PID
let pid = std::process::id();
fs::write(&pid_path, pid.to_string())?;

// Read PID
let pid: u32 = fs::read_to_string(&pid_path)?.trim().parse()?;

// Clean up
fs::remove_file(&pid_path)?;
```

`std::process::id()` returns the current process ID as `u32`. Parsing from string requires `.trim()` because the file may contain a trailing newline.

**Stale PID detection**: Before writing, check if the PID in the file (if it exists) belongs to a live process. If not, it's stale -- remove it and continue.

## Gotchas

- Always check for stale PID files. A crash leaves the file behind.
- `fs::write` is atomic on most OSes for small payloads (one PID string fits in a single page).
- Use `let _ = fs::remove_file(...)` during cleanup -- don't fail on missing file.

## Links

- [007-process-liveness-check](./007-process-liveness-check.md)
