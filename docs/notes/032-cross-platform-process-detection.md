---
id: 032
title: "Cross-Platform Process Detection by Name"
tags: [rust, windows, linux, process, platform]
phase: 4
created: 2026-06-09
---

## What

Finding a running process by its executable name without knowing its PID. This requires platform-specific code: Windows uses the ToolHelp32 API, Linux scans `/proc`.

## Why

`bnvr kernel status` needs to report whether Mihomo is running and its PID. The daemon's kernel manager also checks if Mihomo is already running before spawning a duplicate. Both need "find process by name" functionality.

## How

**Windows -- ToolHelp32 snapshot:**

```rust
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
};

unsafe {
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    let mut entry: PROCESSENTRY32 = std::mem::zeroed();
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    if Process32First(snapshot, &mut entry) != 0 {
        loop {
            let name = CStr::from_ptr(entry.szExeFile.as_ptr() as *const c_char);
            if name.to_string_lossy().contains("mihomo") {
                return Some(entry.th32ProcessID);
            }
            if Process32Next(snapshot, &mut entry) == 0 { break; }
        }
    }
}
```

**Linux -- /proc scan:**

```rust
for entry in fs::read_dir("/proc")?.flatten() {
    let pid: u32 = entry.file_name().to_string_lossy().parse().ok()?;
    if let Ok(exe) = fs::read_link(entry.path().join("exe")) {
        if exe.file_name()?.to_string_lossy().contains("mihomo") {
            return Some(pid);
        }
    }
}
```

Key gotchas:
- Windows: `szExeFile` is `[u8; 260]` (MAX_PATH), null-terminated. Use `CStr::from_ptr` to convert.
- Windows: `PROCESSENTRY32.dwSize` must be set before calling `Process32First` or it fails silently.
- Linux: reading `/proc/<pid>/exe` requires permission. If the process belongs to another user, `read_link` returns `Err`.
- The Cargo.toml needs `Win32_System_Diagnostics_ToolHelp` feature (not `Win32_System_Threading`).

## Links

- [020-os-arch-detection](./020-os-arch-detection.md)
- [017-windows-handle-null-check](./017-windows-handle-null-check.md)
