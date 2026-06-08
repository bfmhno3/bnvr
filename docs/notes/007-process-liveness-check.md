---
id: 007
title: "Process Liveness Check"
tags: [rust, process, platform, unsafe]
phase: 2
created: 2026-06-08
---

## What

Checking if a process with a given PID is still running. Platform-specific: `kill(2)` on Unix, `OpenProcess` on Windows.

## Why

PID files can be stale after a crash. Before starting a daemon or killing one, we need to verify the process actually exists.

## How

```rust
// Unix: signal 0 tests existence without sending a signal
unsafe { libc::kill(pid as i32, 0) == 0 }

// Windows: open with query permission, check handle, close
unsafe {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if handle.is_null() { return false; }
    let _ = CloseHandle(handle);
    true
}
```

`kill(pid, 0)` returns 0 if the process exists and we have permission, -1 otherwise. It does not send any signal.

On Windows, `OpenProcess` returns a null handle if the process doesn't exist or access is denied. We must close the handle after checking.

**`handle.is_null()`** not `handle == 0`. Windows `HANDLE` is `*mut c_void`, not an integer. Comparing to `0` (usize) is a type error. Use `.is_null()` for pointer null checks.

## Gotchas

- On Unix, `kill(0)` returns false for processes we don't own (EPERM). For a daemon running as the same user, this is fine.
- PID reuse: a PID can be recycled. The window is small but nonzero. For robustness, combine with a timestamp or unique token in the PID file.
- Always close Windows handles. Leaked handles keep the process object alive.
- `HANDLE` in `windows-sys` is `*mut c_void`. Integer comparisons (`== 0`) don't compile. Use `.is_null()`.

## Links

- [005-pid-file-management](./005-pid-file-management.md)
- [017-windows-handle-null-check](./017-windows-handle-null-check.md)
