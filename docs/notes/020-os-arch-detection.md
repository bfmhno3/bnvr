---
id: 020
title: "OS and Architecture Detection"
tags: [rust, cross-platform, cfg]
phase: 4
created: 2026-06-08
---

## What

Using `cfg!()` macros and `std::env::consts` to detect the current OS and CPU architecture at compile time.

## Why

Mihomo releases have different binary names per platform (`mihomo-windows-amd64`, `mihomo-linux-arm64`, etc.). BNVR must pick the right one.

## How

```rust
fn detect_platform() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "windows") { "windows" }
             else if cfg!(target_os = "linux") { "linux" }
             else { panic!("unsupported OS") };

    let arch = if cfg!(target_arch = "x86_64") { "amd64" }
               else if cfg!(target_arch = "aarch64") { "arm64" }
               else { panic!("unsupported arch") };

    (os, arch)
}
```

Key points:
- `cfg!()` evaluates at compile time, returns `bool`
- `std::env::consts::OS` and `ARCH` are runtime alternatives but less idiomatic for this use case
- Mapping: `x86_64` -> `amd64`, `aarch64` -> `arm64` (Mihomo naming)

## Links

- [004-match-exhaustive-destructuring](./004-match-exhaustive-destructuring.md)
