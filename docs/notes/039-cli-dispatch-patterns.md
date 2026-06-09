---
id: 039
title: "Building CLI Dispatch Patterns"
tags: [rust, cli, clap, architecture]
phase: 5
created: 2026-06-09
---

## What

The pattern of matching on clap subcommand enums in main.rs and dispatching to module functions. The CLI layer should be thin: parse arguments, call library functions, format output.

## Why

BNVR's main.rs dispatches 8 top-level commands and 20+ subcommands. Keeping the CLI layer thin makes commands testable (library functions can be unit-tested) and main.rs readable.

## How

```rust
match cli.command {
    Some(Commands::Profile { action }) => {
        let conn = daemon::db::open()?;
        match action {
            ProfileAction::Add { url, name } => {
                profile::crud::add(&conn, &name, &url)?;
                println!("profile '{}' added", name);
            }
            ProfileAction::List => {
                let profiles = profile::crud::list(&conn)?;
                for p in &profiles {
                    println!("  {} {}", p.name, p.url);
                }
            }
            // ...
        }
    }
}
```

**Async vs sync in the same match block**: Under `#[tokio::main]`, both sync and async calls work. Sync calls block the current task; async calls yield to the runtime. For short operations (DB queries), sync is fine. For I/O-bound operations (HTTP fetch), use `.await`.

**Error handling**: Two patterns:
1. `?` operator when main returns `Result`
2. `eprintln! + process::exit(1)` when main returns `()`

## Links

- [002-clap-derive-macros](./002-clap-derive-macros.md) - clap CLI definition
- [004-match-exhaustive-destructuring](./004-match-exhaustive-destructuring.md) - match patterns
