---
id: 002
title: "Clap Derive Macros"
tags: [rust, cli, clap, derive]
phase: 1
created: 2026-06-08
---

## What

clap's derive mode lets you define CLI structure as Rust structs and enums. You annotate types with `#[derive(Parser)]` and `#[derive(Subcommand)]`, and clap generates the argument parser from the type definitions.

## Why

BNVR has a 2-level command matrix (`bnvr daemon start`, `bnvr kernel list`, etc.). Hand-writing this with string matching would be error-prone and tedious. Derive macros give you type-safe subcommands with auto-generated help text.

## How

```rust
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "bnvr", version, about = "BNVR is Not Verge Rev")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,  // Option = no subcommand is valid (TUI mode)
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    // ...
}

#[derive(Subcommand, Debug)]
pub enum DaemonAction {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status,
}
```

Usage in main:

```rust
let cli = Cli::parse();
match cli.command {
    None => println!("TUI mode"),
    Some(Commands::Daemon { action }) => match action {
        DaemonAction::Start => { /* ... */ }
        DaemonAction::Stop => { /* ... */ }
        DaemonAction::Status => { /* ... */ }
    },
    // ...
}
```

## Key patterns

- **`Option<Commands>`** -- when no subcommand is given, `cli.command` is `None`. This is how `bnvr` (no args) triggers TUI mode.
- **Nested enums** -- each subcommand category (`Daemon`, `Kernel`, etc.) contains its own enum for second-level actions.
- **Doc comments (`///`)** -- become the help text. `bnvr daemon --help` shows them.
- **`#[arg(long)]`** -- for named arguments like `--out`. Positional args are just struct fields without `#[arg(...)]`.

## Gotchas

- Every arm of the nested `match` must be handled. The compiler enforces exhaustiveness -- if you add a variant to the enum and forget to handle it, it won't compile.
- The derive macros generate a `parse()` method. If you call `Cli::parse()` in main, clap handles `--help` and `--version` automatically (and exits the process).

## Links

- [001-cargo-toml-dependencies](./001-cargo-toml-dependencies.md)
