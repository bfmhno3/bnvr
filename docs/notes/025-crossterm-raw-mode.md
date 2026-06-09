---
id: 025
title: "Crossterm Raw Mode and Alternate Screen"
tags: [rust, tui, crossterm, terminal]
phase: 3
created: 2026-06-09
---

## What

Crossterm provides cross-platform terminal manipulation. Raw mode disables line buffering and echo; alternate screen switches to a separate terminal buffer so the user's shell history is preserved.

## Why

BNVR's TUI needs full keyboard control (single keypresses, no Enter required) and must not pollute the user's terminal. Raw mode + alternate screen achieves both.

## How

```rust
use crossterm::{
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};

// Enter
enable_raw_mode()?;
execute!(io::stdout(), EnterAlternateScreen)?;

// ... run TUI ...

// Exit -- MUST restore or terminal is broken
disable_raw_mode()?;
execute!(io::stdout(), LeaveAlternateScreen)?;
```

Gotchas:
- If the process crashes without restoring, the terminal is left in raw mode. Fix: set a panic hook (see [029-panic-hook-terminal-restore](./029-panic-hook-terminal-restore.md)).
- `enable_raw_mode()` takes effect globally on the process. Don't call it twice without disabling first.
- On Windows, crossterm uses WinAPI; on Unix, it uses termios.

## Links

- [024-ratatui-basics](./024-ratatui-basics.md)
- [029-panic-hook-terminal-restore](./029-panic-hook-terminal-restore.md)
