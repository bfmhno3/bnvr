---
id: 011
title: "Interprocess Local Socket IPC"
tags: [rust, interprocess, ipc, tokio]
phase: 2
created: 2026-06-08
---

## What

The `interprocess` crate provides cross-platform local sockets. On Linux it uses Unix domain sockets, on Windows it uses Named Pipes. The API is identical on both.

## Why

BNVR needs IPC between the CLI client and the daemon. Implementing Unix sockets and Named Pipes separately means two codepaths. `interprocess` unifies them.

## How

```rust
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, ToNsName};
use interprocess::local_socket::traits::tokio::{Listener as _, Stream as _};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
```

**Socket naming** -- use `to_ns_name` with `GenericNamespaced`:
```rust
let name = "bnvr".to_ns_name::<GenericNamespaced>()?;
let listener = ListenerOptions::new().name(name).create_tokio()?;
```

**NOT** `"bnvr".into()` -- `Name` doesn't implement `From<&str>`. You must use the `ToNsName` trait.

**NOT** `Listener::bind(opts)` -- use `ListenerOptions::new().name(name).create_tokio()`.

**Trait imports** -- `accept()` and `split()` are on trait impls, not inherent methods:
```rust
use interprocess::local_socket::traits::tokio::{Listener as _, Stream as _};
```

Without these imports, `listener.accept()` and `stream.split()` won't compile.

**Connection handling**:
```rust
loop {
    let stream = listener.accept().await?;
    tokio::spawn(async move {
        handle_connection(stream).await;
    });
}
```

**Stream I/O** -- `split()` returns `(RecvHalf, SendHalf)`. `RecvHalf` implements `AsyncRead`, `SendHalf` implements `AsyncWrite`:
```rust
let (recv_half, send_half) = stream.split();
let mut reader = BufReader::new(recv_half);
// read_line, write_all work normally
```

The socket name `"bnvr"` maps to:
- Linux: `/tmp/BNVR.bnvr` (filesystem path)
- Windows: `\\.\pipe\bnvr` (named pipe)

## Gotchas

- The `tokio` feature must be enabled: `interprocess = { version = "2", features = ["tokio"] }`.
- Three trait imports are easy to miss: `ToNsName`, `Listener as _`, `Stream as _`. Without them you get "method not found" errors.
- `create_tokio()` not `bind()`. The API changed in interprocess 2.x.
- On Linux, the socket file is left behind if the daemon crashes. Clean up stale socket files on startup.
- On Windows, named pipes are cleaned up automatically when the handle closes.
- `accept()` returns a new `Stream` per connection. Each needs its own task.

## Links

- [010-json-message-protocol](./010-json-message-protocol.md)
- [009-tokio-shutdown-signal](./009-tokio-shutdown-signal.md)
