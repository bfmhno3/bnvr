---
id: 037
title: "ANSI Terminal Colors in Rust"
tags: [rust, terminal, ansi, cli]
phase: 5
created: 2026-06-09
---

## What

ANSI escape codes are sequences of bytes that control terminal text formatting. Common codes: `\x1b[31m` (red), `\x1b[32m` (green), `\x1b[0m` (reset).

## Why

`bnvr profile diff` shows added/removed lines with color. Using manual ANSI codes avoids adding a dependency like the `colored` crate.

## How

```rust
println!("\x1b[32m+ added line\x1b[0m");   // green
println!("\x1b[31m- removed line\x1b[0m"); // red
println!("  unchanged line");              // default
```

Common codes:
- `\x1b[30m` - black, `\x1b[31m` - red, `\x1b[32m` - green, `\x1b[33m` - yellow
- `\x1b[34m` - blue, `\x1b[35m` - magenta, `\x1b[36m` - cyan, `\x1b[37m` - white
- `\x1b[0m` - reset all formatting

**Windows note**: Windows Terminal (Windows 11) supports ANSI natively. Older cmd.exe does not. For broad compatibility, the `colored` crate auto-detects terminal capabilities.

## Links

- [025-crossterm-raw-mode](./025-crossterm-raw-mode.md) - crossterm for TUI terminal control
