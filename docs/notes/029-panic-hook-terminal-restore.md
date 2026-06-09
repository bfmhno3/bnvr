---
id: 029
title: "Panic Hook for Terminal Restoration"
tags: [rust, tui, panic, safety]
phase: 3
created: 2026-06-09
---

## What

A panic hook that restores the terminal (disables raw mode, leaves alternate screen) before the default panic handler runs. Without this, a panic leaves the terminal in a broken state.

## Why

If BNVR's TUI panics while in raw mode, the user's terminal becomes unusable -- no echo, no line buffering, invisible input. The panic hook is a safety net.

## How

```rust
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    original_hook(info); // still print the panic message
}));
```

Key points:
- `take_hook()` saves the default hook (prints panic location + backtrace)
- Call the original hook after restoring the terminal, so the panic message is visible
- Use `let _ =` because we can't handle errors in a panic hook -- best effort restoration
- Set the hook AFTER enabling raw mode, so it only fires when the TUI is active
- The hook is process-global; if you have multiple TUI sessions, coordinate accordingly

## Links

- [025-crossterm-raw-mode](./025-crossterm-raw-mode.md)
- [024-ratatui-basics](./024-ratatui-basics.md)
