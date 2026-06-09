---
id: 026
title: "TUI Event Loop with Tokio"
tags: [rust, tui, async, tokio, crossterm]
phase: 3
created: 2026-06-09
---

## What

A TUI event loop polls for keyboard input and periodic ticks, sending events to the main app loop via a channel. Using tokio's mpsc channel decouples input polling from rendering.

## Why

BNVR's TUI must handle keypresses (j/k, q, Tab) and periodic updates (future: live data from daemon). A channel-based design keeps the event poller separate from the state updater and renderer.

## How

```rust
use tokio::sync::mpsc;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

enum AppEvent {
    Key(KeyEvent),
    Tick,
}

async fn run_event_loop(tx: mpsc::Sender<AppEvent>) {
    loop {
        if event::poll(Duration::from_millis(200)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if tx.send(AppEvent::Key(key)).await.is_err() {
                    return; // receiver dropped
                }
            }
        } else if tx.send(AppEvent::Tick).await.is_err() {
            return;
        }
    }
}
```

The main loop receives events and updates state:
```rust
let (tx, mut rx) = mpsc::channel(32);
tokio::spawn(run_event_loop(tx));

loop {
    terminal.draw(|frame| render(frame, &state))?;
    if let Some(event) = rx.recv().await {
        match event {
            AppEvent::Key(key) => handle_key(key, &mut state),
            AppEvent::Tick => {}
        }
    }
    if state.should_quit { break; }
}
```

Key points:
- `event::poll()` is non-blocking with a timeout -- yields every 200ms for ticks
- Channel capacity of 32 is plenty; keypresses are fast
- The event loop task exits when the receiver is dropped (clean shutdown)

## Links

- [025-crossterm-raw-mode](./025-crossterm-raw-mode.md)
- [028-tui-state-machine](./028-tui-state-machine.md)
