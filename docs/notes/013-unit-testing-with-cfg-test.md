---
id: 013
title: "Unit Testing with #[cfg(test)]"
tags: [rust, testing, cfg]
phase: 2
created: 2026-06-08
---

## What

`#[cfg(test)]` marks a module that only compiles when running `cargo test`. Unit tests live inside the same file as the code they test, at the bottom.

## Why

Tests need access to private functions and internals. Putting them in the same file with `#[cfg(test)]` keeps them close to the code and gives them access to private items.

## How

```rust
// src/daemon/process.rs

pub fn is_alive(pid: u32) -> bool {
    // ... implementation
}

#[cfg(test)]
mod tests {
    use super::*;  // import everything from parent module

    #[test]
    fn test_current_process_is_alive() {
        let pid = std::process::id();
        assert!(is_alive(pid));
    }

    #[test]
    fn test_invalid_pid_is_not_alive() {
        assert!(!is_alive(u32::MAX - 1));
    }
}
```

**`#[cfg(test)]`** means the module is compiled only during `cargo test`, not `cargo build`. Zero binary size cost.

**`use super::*`** imports everything from the parent module -- including private items. This is why unit tests can test private functions.

**`#[test]`** marks a function as a test. `cargo test` discovers and runs it.

## Patterns

**Testing error cases**:
```rust
#[test]
fn test_send_signal_invalid_pid() {
    let result = send_shutdown_signal(u32::MAX - 1);
    assert!(result.is_err());
}
```

**Testing with temp data**:
```rust
#[test]
fn test_ensure_dirs() {
    ensure_dirs().unwrap();
    assert!(bnvr_home().exists());
}
```

**`assert_eq!` for values, `assert!` for conditions, `.is_err()` for error paths.**

## Gotchas

- `#[cfg(test)]` code is not included in release builds. Don't put runtime logic there.
- `use super::*` pulls in everything. If two modules have items with the same name, you'll get ambiguity errors.
- Each `#[test]` function runs in its own thread. Tests that modify shared state (files, env vars) can interfere with each other.
- `cargo test -- --show-output` prints `println!` output from passing tests.

## Links

- [014-integration-tests](./014-integration-tests.md)
- [015-in-memory-sqlite-testing](./015-in-memory-sqlite-testing.md)
