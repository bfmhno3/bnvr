---
id: 024
title: "Ratatui Basics"
tags: [rust, tui, ratatui]
phase: 3
created: 2026-06-09
---

## What

Ratatui is a Rust library for building terminal user interfaces. It provides a declarative API for rendering widgets (text, blocks, tables, charts) to a terminal backend.

## Why

BNVR needs a TUI to display node status, traffic data, and logs. Ratatui is the standard Rust TUI library (successor to `tui-rs`), actively maintained, and works well with crossterm for cross-platform terminal control.

## How

The core loop is: create a `Terminal`, then in a loop call `terminal.draw(|frame| ...)` where `frame` lets you render widgets into rectangular areas.

```rust
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui::widgets::{Block, Borders, Paragraph};

let backend = CrosstermBackend::new(io::stdout());
let mut terminal = Terminal::new(backend)?;

terminal.draw(|frame| {
    let block = Block::default().title("Hello").borders(Borders::ALL);
    frame.render_widget(block, frame.area());
})?;
```

Key types:
- `Terminal<B>` -- owns the backend, handles buffer diffing
- `Frame` -- passed to the draw closure, provides `render_widget()` and `area()`
- `Widget` -- trait implemented by Block, Paragraph, Table, etc.
- `Layout` -- splits a `Rect` into sub-areas with constraints

## Links

- [025-crossterm-raw-mode](./025-crossterm-raw-mode.md)
- [027-ratatui-layout](./027-ratatui-layout.md)
