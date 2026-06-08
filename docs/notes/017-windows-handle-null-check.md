---
id: 017
title: "Windows HANDLE Type and Null Checks"
tags: [rust, windows, unsafe, ffi]
phase: 2
created: 2026-06-08
---

## What

Windows API uses `HANDLE` for process handles, file handles, etc. In `windows-sys`, `HANDLE` is `*mut c_void` (a raw pointer), not an integer. Null checks use `.is_null()`, not `== 0`.

## Why

`OpenProcess` returns a null handle when it fails. Checking for failure requires the right comparison. Using `== 0` on a `*mut c_void` is a type error.

## How

```rust
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

unsafe {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if handle.is_null() {
        return false;
    }
    let _ = CloseHandle(handle);
    true
}
```

**`handle.is_null()`** -- the correct way to check for null pointer. Works on any `*const T` or `*mut T`.

**`handle == 0`** -- does NOT compile. `0` is `usize`, `handle` is `*mut c_void`. Different types.

**`std::ptr::null_mut()`** -- if you need an explicit null pointer value:
```rust
if handle == std::ptr::null_mut() { ... }
```
But `.is_null()` is shorter and idiomatic.

## The error we got

```
error[E0308]: mismatched types
  |
  |     if handle == 0 {
  |        ------    ^ expected `*mut c_void`, found `usize`
```

The compiler expects `*mut c_void` on both sides of `==`. `0` is `usize`. Fix: `handle.is_null()`.

## Gotchas

- `windows-sys` types are raw pointers. Integer comparisons don't work.
- Always close handles with `CloseHandle`. Leaked handles keep the kernel object alive.
- `HANDLE` being `*mut c_void` means it's nullable. Use `.is_null()` before dereferencing.
- On Unix, process handles are just integers (PIDs). No pointer semantics. This is a Windows-specific gotcha.

## Links

- [007-process-liveness-check](./007-process-liveness-check.md)
