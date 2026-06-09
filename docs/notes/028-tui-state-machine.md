---
id: 028
title: "TUI State Machine Pattern"
tags: [rust, tui, architecture]
phase: 3
created: 2026-06-09
---

## What

A single `AppState` struct holds all mutable TUI state. The event loop mutates it; the render function reads it. This is the simplest architecture for a TUI app.

## Why

BNVR's TUI needs to track which panel is focused, whether to quit, and daemon connection status. A central state struct avoids passing data through multiple layers.

## How

```rust
pub struct AppState {
    pub focused: FocusedPanel,
    pub should_quit: bool,
    pub daemon_connected: bool,
}

impl AppState {
    pub fn new() -> Self { /* defaults */ }
    pub fn focus_next(&mut self) { self.focused = self.focused.next(); }
    pub fn quit(&mut self) { self.should_quit = true; }
}
```

The pattern is: `event -> mutate state -> render from state`. No callbacks, no observers, no event bus. The main loop owns the state and passes `&state` to the render function.

For larger apps, this scales to an enum-based state machine:
```rust
enum AppMode {
    Normal,
    CommandInput { buffer: String },
    ConfirmQuit,
}
```

But for Phase 3, a flat struct is enough.

## Links

- [026-tui-event-loop](./026-tui-event-loop.md)
- [027-ratatui-layout](./027-ratatui-layout.md)
