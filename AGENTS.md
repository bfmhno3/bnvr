# Agents Instruction: BNVR

BNVR means "BNVR is Not Verge Rev". It is a Rust command-line network-management program designed as a lean alternative to the bulky Clash Verge Rev GUI. It runs on Windows and Linux, manages Mihomo kernels, provides a TUI client, and supports isolated Python overwrite plugins.

## Project Overview

Source follows the command tree:

```text
src/
  main.rs        Tokio entrypoint and command dispatch
  cli.rs         clap command definitions
  lib.rs         reusable module exports
  paths.rs       current shared path helper module
  daemon/        bnvr daemon ...
  kernel/        bnvr kernel ...
  profile/       bnvr profile ...
  overwrite/    bnvr overwrite ...
  tui/           default bnvr terminal client
```

`src/main.rs` dispatches the clap tree from `src/cli.rs`. Each command folder under `src/` should match a second-level CLI command, such as `bnvr daemon` mapping to `src/daemon/`. Files inside a command folder should normally match the final command layer, for example `start.rs`, `stop.rs`, `status.rs`, `list.rs`, or `use.rs`; `crud.rs` is allowed when the code is shared CRUD rather than one command action.

`src/daemon/` owns lifecycle, local-socket JSON IPC, bundled SQLite through rusqlite, process monitoring, and tracing. `src/kernel/` detects supported platforms, downloads Mihomo, and selects active kernels. `src/profile/` handles subscription CRUD, sync, view, and diff. `src/overwrite/` handles plugin CRUD, Git passthrough, and the three-second JSON stdin/stdout Python bridge. `src/tui/` is the ratatui/crossterm client.

`src/paths.rs` is the only current common module. If another common module is added, move all common modules into `src/utilities/` and update exports and callers together.

Integration tests live under `tests/`. Unit tests are colocated in `#[cfg(test)]` modules. `docs/design_guide.md` records direction and `TODO.md` may lag source; verify behavior in source and tests.

## Environment Configuration

- Use rustup with the project `rust-toolchain.toml`; it selects the latest stable Rust toolchain and includes rustfmt and Clippy.
- Windows and Linux are supported. macOS is unsupported by `src/kernel/download.rs::detect_platform`.
- Use `BNVR_HOME` to isolate test and development state from the default `~/.bnvr`.
- Generated state under `BNVR_HOME`: `bnvr.pid`, `bnvr.db`, `logs/`, `kernels/`, `overwrite/`.
- Optional `GITHUB_TOKEN` authenticates Mihomo GitHub release requests. Never commit credentials.
- Optional Python plus `uv` creates and runs overwrite-plugin virtual environments.
- Optional `git` enables overwrite Git passthrough and related integration tests.
- Keep rustfmt and Clippy installed through rustup components.

## Build and Test Commands

Run from the repository root:
```sh
rustup update stable
rustup show
rustup component add rustfmt clippy
```


```sh
cargo build
cargo test --all-targets
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- --help
cargo run -- tldr
```

## Tools Usage

- Read relevant files before editing. Search for existing implementation before adding one.
- Use ast-grep for structural code search. If ast-grep is unavailable, stop and tell the user instead of continuing with weaker structural search.
- Keep changes scoped. Do not add speculative features, boilerplate, compatibility shims, or single-use abstractions.
- Use rustup to manage the stable Rust toolchain and components. Use Cargo for build, test, format, and lint.
- Use clap derive for commands, Tokio for async work, `tracing` structured fields for logs, rusqlite placeholders such as `?1` and `?2` for SQL, reqwest with user agent `bnvr` for HTTP, and `src/paths.rs` for paths.
- Write Git commit messages with the Conventional Commits format, such as `feat: add kernel status command` or `fix: handle missing plugin venv`.
- For CLI work, update both `src/cli.rs` and `src/main.rs`. Preserve the two-level command layout.
- For exported symbol changes, update every source and test caller.
- For user-state tests, set `BNVR_HOME` to a unique temporary directory and clean it up. Serialize process-global environment mutation with a mutex. Never test against the real `~/.bnvr`.
- For Python hooks, preserve `HookRequest`, `HookResponse`, and `HOOK_TIMEOUT = Duration::from_secs(3)` in `src/overwrite/bridge.rs`. Report timeout, subprocess, nonzero exit, invalid JSON, and plugin-reported errors.

## Code Style

- Use `snake_case` for modules, functions, and tests; `PascalCase` for structs and enums; `SCREAMING_SNAKE_CASE` for constants.
- Keep modules focused by command/domain. Use exhaustive `match` for command and state dispatch.
- Use `Result<_, Box<dyn std::error::Error>>` where surrounding code already does. Propagate with `?`. Match nearby lowercase error-message style.
- Keep the simplest working solution. Do not over-engineer, abstract one use, add speculative behavior, handle impossible cases, or refactor three similar lines prematurely.
- Read before modifying. Do not add doc comments, docstrings, or type annotations to untouched code.
- Comments are sparse and explain only non-obvious safety, lifecycle, or platform behavior.
- Preserve structured logging such as `info!(name = %name, bytes, "sync complete")`. Never log tokens, subscription secrets, full sensitive configs, or credentials.
- Keep SQL parameterized and platform branches explicit with `cfg!` or `#[cfg]`.
- Tests assert observable behavior, use `test_<behavior>` names, and use `#[test]` or `#[tokio::test]` as appropriate. Conditional skips belong only in integration tests for optional external executables.

### Output

Return code first. Explain only non-obvious details after code. Do not add inline prose or unrequested boilerplate. Keep code copy-paste safe.

### Review

State the bug. Show the fix. Stop. No compliments. No out-of-scope suggestions.

### Debugging

Read relevant code first. State what you found, where, and the fix in one pass. If the cause is unclear, say the cause is unclear and do not guess.

### Formatting

Use plain ASCII punctuation unless natural-language content requires Unicode. Do not use em dashes, smart quotes, or decorative Unicode. Use plain hyphens and straight quotes.
