---
id: 004
title: "Match and Exhaustive Destructuring"
tags: [rust, pattern-matching, enums]
phase: 1
created: 2026-06-08
---

## What

`match` is Rust's pattern matching expression. Unlike `switch` in C/JS, it is **exhaustive** -- the compiler forces you to handle every variant of an enum. If you miss one, it won't compile.

## Why

The CLI dispatch in `main.rs` is a nested match: `Commands` -> `DaemonAction` / `KernelAction` / etc. Exhaustiveness means you can't accidentally forget to wire up a new subcommand.

## How

```rust
match cli.command {
    None => println!("TUI mode"),
    Some(cmd) => match cmd {
        Commands::Daemon { action } => match action {
            DaemonAction::Start => { /* ... */ }
            DaemonAction::Stop => { /* ... */ }
            DaemonAction::Status => { /* ... */ }
        },
        Commands::Kernel { action } => match action {
            KernelAction::List => { /* ... */ }
            KernelAction::Install => { /* ... */ }
            KernelAction::Use { version } => println!("use {}", version),
            KernelAction::Status => { /* ... */ }
        },
        // ... every Commands variant must be handled
    },
}
```

## Destructuring

Match arms destructure the enum variant and bind its fields:

```rust
KernelAction::Use { version } => println!("use {}", version),
//                  ^^^^^^^^ binds the `version: String` field

ProfileAction::Add { url, name } => println!("add {} {}", url, name),
//                 ^^^^^^^^^^^^ binds both fields
```

This is the same syntax as struct patterns. Enums with named fields (like `Add { url, name }`) use `{}`. Tuple variants use `()`.

## Exhaustiveness is a feature

If you add a new variant to `KernelAction`:

```rust
pub enum KernelAction {
    List,
    Install,
    Use { version: String },
    Status,
    Upgrade,  // new!
}
```

The compiler will refuse to build until you handle `Upgrade` in every `match` that covers `KernelAction`. This catches wiring bugs at compile time.

## Gotchas

- `_` is a catch-all pattern. Use it only when you truly don't care about the value. Overusing `_` defeats exhaustiveness.
- Nested matches get deep. The design guide's 2-level command structure (domain + action) keeps it manageable. Three levels would be painful.

## Links

- [002-clap-derive-macros](./002-clap-derive-macros.md)
