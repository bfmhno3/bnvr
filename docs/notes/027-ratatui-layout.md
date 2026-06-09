---
id: 027
title: "Ratatui Layout and Constraints"
tags: [rust, tui, ratatui, layout]
phase: 3
created: 2026-06-09
---

## What

Ratatui's `Layout` splits a rectangular area into sub-areas using constraints. This is how you build multi-panel TUIs.

## Why

BNVR's TUI has a header, three side-by-side panels (Nodes, Traffic, Logs), and a status bar. Layout handles the splitting without manual math.

## How

```rust
use ratatui::layout::{Constraint, Direction, Layout};

// Vertical split: header | main | status
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(1),  // header: exactly 1 row
        Constraint::Min(5),    // main: at least 5 rows, takes remaining space
        Constraint::Length(1), // status bar: exactly 1 row
    ])
    .split(frame.area());

// Horizontal split: 3 panels
let panels = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .split(chunks[1]); // split the "main" area
```

Constraint types:
- `Length(n)` -- exactly n rows/columns
- `Min(n)` -- at least n, grows to fill remaining space
- `Percentage(p)` -- p% of the parent area
- `Ratio(num, den)` -- num/den of the parent

`split()` returns `Vec<Rect>`. Index into it to get each sub-area.

## Links

- [024-ratatui-basics](./024-ratatui-basics.md)
- [028-tui-state-machine](./028-tui-state-machine.md)
