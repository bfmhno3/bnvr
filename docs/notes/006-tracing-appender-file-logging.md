---
id: 006
title: "Tracing Appender File Logging"
tags: [rust, tracing, logging]
phase: 2
created: 2026-06-08
---

## What

`tracing-appender` writes tracing output to files instead of (or alongside) stderr. `rolling::daily` creates a new log file each day.

## Why

The daemon runs in the background with no terminal. Logs must go to files. `~/.bnvr/logs/bnvr.YYYY-MM-DD` keeps logs organized.

## How

```rust
use tracing_subscriber::fmt::writer::MakeWriterExt;

let file_appender = tracing_appender::rolling::daily(&log_dir, "bnvr.log");
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

tracing_subscriber::fmt()
    .with_env_filter(...)
    .with_writer(std::io::stderr.and(non_blocking))
    .init();

std::mem::forget(_guard);
```

**`MakeWriterExt`** is the trait that provides `.and()`. You must import it:
```rust
use tracing_subscriber::fmt::writer::MakeWriterExt;
```
Without this import, `std::io::stderr.and(non_blocking)` won't compile.

**`non_blocking`** spawns a background thread that flushes writes. Without it, every log line blocks on disk I/O.

**`.and(non_blocking)`** writes to both stderr and the file simultaneously. The return type is `Tee<Stderr, NonBlocking>`.

**`_guard`** must live for the process lifetime. `mem::forget` leaks it intentionally -- on process exit the OS reclaims it.

## Gotchas

- `tracing_subscriber::fmt().init()` can only be called once per process. Calling it again panics.
- `non_blocking` drops log lines if the background channel is full. Acceptable for a daemon; not for audit logs.
- `rolling::daily` uses UTC by default. Check if you need local time.
- The import `use tracing_subscriber::fmt::writer::MakeWriterExt` is easy to miss. Without it you get "method `and` not found".

## Links

- [003-tracing-structured-logging](./003-tracing-structured-logging.md)
