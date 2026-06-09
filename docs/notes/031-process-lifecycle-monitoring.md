---
id: 031
title: "Child Process Lifecycle Monitoring with Auto-Restart"
tags: [rust, tokio, process, supervision]
phase: 4
created: 2026-06-09
---

## What

A background tokio task that watches a child process, detects when it exits, and automatically restarts it. This is the "supervisor" pattern -- keep a critical process alive without manual intervention.

## Why

Mihomo is the core network kernel. If it crashes, the proxy stops working. BNVR's daemon needs to detect the crash and restart Mihomo within seconds, without requiring the user to notice and manually intervene.

## How

```rust
fn spawn_monitor(inner: Arc<Mutex<KernelState>>) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            // Wait until we have a child
            let wait_result = {
                let mut state = inner.lock().await;
                match state.child.as_mut() {
                    Some(child) => Some(child.wait().await),
                    None => None,
                }
            };

            let result = match wait_result {
                Some(r) => r,
                None => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Clean up dead child
            {
                let mut state = inner.lock().await;
                state.child = None;
            }

            // Backoff before restart
            warn!("restarting in 3 seconds...");
            tokio::time::sleep(Duration::from_secs(3)).await;

            // Re-spawn
            match Command::new(&binary).spawn() {
                Ok(child) => {
                    let mut state = inner.lock().await;
                    state.child = Some(child);
                }
                Err(e) => {
                    error!("restart failed: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    })
}
```

Key design decisions:
- Lock the mutex, call `child.wait()`, release the lock before restarting. This avoids holding the lock during the (potentially long) wait.
- Use a `restart_on_crash` flag to allow disabling auto-restart
- Backoff (3s) prevents crash loops from consuming CPU
- The monitor task owns the restart logic; the KernelManager only starts/stops

## Links

- [030-child-process-spawn-tokio](./030-child-process-spawn-tokio.md)
- [016-thread-safe-error-types](./016-thread-safe-error-types.md)
