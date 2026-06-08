---
id: 021
title: "File-Based Configuration Pattern"
tags: [rust, design-pattern, filesystem]
phase: 4
created: 2026-06-08
---

## What

Storing simple configuration values as plain text files on disk (one value per file), rather than using a config format like TOML or JSON.

## Why

BNVR stores the active kernel version as a single string in `~/.bnvr/kernels/.active`. A full config file would be overkill. Plain text files are trivially readable, writable, and diffable.

## How

```rust
// Write
std::fs::write(paths::active_kernel_file(), "v1.19.27")?;

// Read
let version = std::fs::read_to_string(paths::active_kernel_file())
    .ok()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty());
```

Key points:
- File name starts with `.` to hide it from directory listings
- Always `trim()` when reading (editor may add trailing newline)
- Use `.filter(|s| !s.is_empty())` to handle empty file gracefully
- No locking needed -- single-writer pattern (only CLI or daemon writes, not both)

## Links

- [005-pid-file-management](./005-pid-file-management.md) -- same pattern for PID file
- [008-rusqlite-database-setup](./008-rusqlite-database-setup.md) -- contrast with DB storage
