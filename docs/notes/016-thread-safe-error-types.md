---
id: 016
title: "Thread-Safe Error Types"
tags: [rust, error-handling, concurrency, tokio]
phase: 2
created: 2026-06-08
---

## What

`Box<dyn std::error::Error>` is not `Send + Sync`. It can't be returned from `tokio::spawn` or moved across threads. You need `Box<dyn std::error::Error + Send + Sync>` or convert to `String`.

## Why

`tokio::spawn` requires the future's output to be `Send` (movable across threads). `Box<dyn Error>` contains trait objects that may not be `Send`. The compiler rejects it.

## How

**Option 1: Constrain the error type in function signatures**:
```rust
fn my_function() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // ...
}
```

**Option 2: Convert to String before spawning**:
```rust
tokio::spawn(async move {
    some_async_fn().await.map_err(|e| e.to_string())
});
```

**Option 3: Use a concrete error type**:
```rust
#[derive(Debug)]
enum MyError {
    Io(std::io::Error),
    Json(serde_json::Error),
}
```

## The error we got

```
error[E0277]: `dyn std::error::Error` cannot be sent between threads safely
   --> tokio::spawn(async move { ... })
```

This happens because:
1. `tokio::spawn` requires `F::Output: Send + 'static`
2. `Result<T, Box<dyn Error>>` has `Box<dyn Error>` which is not `Send`
3. `dyn Error` is a trait object -- the compiler can't verify it's `Send`

## The fix

In test helpers that spawn tasks:
```rust
fn start_test_listener(name: &str) -> JoinHandle<Result<(), String>> {
    let name = name.to_string();
    tokio::spawn(async move {
        ipc::listen_on(&name).await.map_err(|e| e.to_string())
    })
}
```

`.map_err(|e| e.to_string())` converts `Box<dyn Error>` to `String`, which is `Send + Sync`.

## Gotchas

- `Box<dyn Error>` is fine for functions that don't cross thread boundaries.
- `tokio::spawn` is the most common place where `Send` matters.
- `async fn` return types must be `Send` if the future is spawned with `tokio::spawn`.
- `.map_err(|e| e.to_string())` loses the original error type. For production code, consider a proper error enum.

## Links

- [014-integration-tests](./014-integration-tests.md)
- [009-tokio-shutdown-signal](./009-tokio-shutdown-signal.md)
