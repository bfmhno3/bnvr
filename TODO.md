# BNVR TODO

## Phase 1: Project Skeleton & CLI Shell

- [x] Add core dependencies to Cargo.toml (clap, tokio, serde, serde_json, tracing, tracing-subscriber, rusqlite, reqwest) ~1d #setup
- [x] Set up clap derive-based CLI with 2-level subcommand structure ~2d #feat
  - [x] Define top-level commands: `daemon`, `kernel`, `profile`, `overwrite`, `network`, `bench`, `stats`, `query`
  - [x] Wire `bnvr` (no args) and `bnvr tldr` stubs
- [x] Set up tracing subscriber for structured logging ~0.5d #feat
- [x] Verify cargo build and cargo test pass ~0.5d #infra

## Phase 2: Daemon Lifecycle (Start/Stop/Status)

- [x] Implement `bnvr daemon start` -- spawn a tokio runtime that stays alive ~2d #feat
  - [x] PID file write to `~/.bnvr/bnvr.pid`
  - [x] Log to file via tracing-appender
- [x] Implement `bnvr daemon stop` -- read PID file, send shutdown signal ~1d #feat
- [x] Implement `bnvr daemon status` -- check if daemon process is alive ~0.5d #feat
- [x] SQLite DB setup (rusqlite) ~1d #feat
  - [x] Create DB at `~/.bnvr/bnvr.db`
  - [x] Schema: profiles, subscriptions, audit_log, bench_results, traffic_stats
- [x] IPC foundation: use interprocess crate for cross-platform local sockets ~2d #feat
  - [x] Define a simple JSON message protocol (request/response)
  - [x] Daemon listens; client connects and sends commands

## Phase 3: TUI Shell (Attach & Detach)

- [x] Add ratatui + crossterm dependencies ~0.5d #setup
- [x] Implement raw mode + alternate screen on `bnvr` (no args) ~1d #feat
- [x] Connect TUI client to daemon socket on startup ~1d #feat
- [x] Implement `q` key to cleanly detach (disable raw mode, leave alt screen) ~0.5d #feat
- [x] Skeleton layout: header, status bar, placeholder panels ~1d #feat
  - [x] Vim-style j/k navigation between panels

## Phase 4: Mihomo Kernel Management

- [x] `bnvr kernel list` -- scan local directory for downloaded kernels ~1d #feat
- [x] `bnvr kernel install` -- download Mihomo binary from GitHub Releases ~2d #feat
  - [x] Detect OS + arch (x86_64/aarch64, windows/linux)
  - [x] Download, extract, place in `~/.bnvr/kernels/`
- [x] `bnvr kernel use <version>` -- switch active kernel version ~1d #feat
- [x] `bnvr kernel status` -- report running kernel PID and version ~0.5d #feat
- [x] Daemon: spawn Mihomo as a child process, monitor its lifecycle ~2d #feat

## Phase 5: Profile (Subscription Management)

- [ ] `bnvr profile add <url> <name>` -- store subscription source ~1d #feat
- [ ] `bnvr profile list` -- show stored subscriptions ~0.5d #feat
- [ ] `bnvr profile del <name>` -- remove subscription ~0.5d #feat
- [ ] `bnvr profile sync [name]` -- fetch YAML from URL, store raw config ~2d #feat
  - [ ] Use reqwest for HTTP fetch
  - [ ] Store in SQLite DB keyed by profile name (rusqlite)
- [ ] `bnvr profile view [json_path]` -- navigate config tree interactively ~1d #feat
- [ ] `bnvr profile diff` -- show before/after overwrite comparison ~1d #feat

## Phase 6: Python Bridge (Overwrite Plugins)

- [ ] `bnvr overwrite init <name>` -- create plugin directory, call `uv venv` ~1d #feat
- [ ] `bnvr overwrite list` / `bnvr overwrite use` -- manage plugins ~0.5d #feat
- [ ] Rust-side stdin/stdout IPC with Python subprocess ~2d #feat
  - [ ] Serialize config dict to JSON, pipe to Python stdin
  - [ ] Read JSON from Python stdout, deserialize back
  - [ ] `tokio::time::timeout` (3s) with forced kill on timeout
- [ ] Implement the 4 hook types: `preprocess`, `postprocess`, `on_node_switch`, `on_network_dropped` ~2d #feat
- [ ] `bnvr overwrite git <args...>` -- transparent git passthrough ~0.5d #feat

## Phase 7: Network Layer (TUN & Routing)

- [ ] `bnvr network tun setup` -- create TUN interface, take over routing ~3d #feat
  - [ ] Windows: wintun + route table manipulation
  - [ ] Linux: tun device + ip route
- [ ] `bnvr network tun clear` -- tear down TUN, restore routes ~1d #feat
  - [ ] Watchdog: auto-clear on daemon crash/panic
- [ ] `bnvr network bypass <ip/cidr>` -- add direct routes ~0.5d #feat

## Phase 8: Benchmarking & Diagnostics

- [ ] `bnvr bench [group]` -- multi-threaded TCP+TLS latency probe ~2d #feat
  - [ ] Measure connect time, TLS handshake, jitter
  - [ ] Write results to SQLite DB
- [ ] `bnvr stats top` -- top domains by traffic ~1d #feat
- [ ] `bnvr stats summary` -- traffic trend chart (TUI sparkline) ~1d #feat
- [ ] `bnvr query rule <domain/ip>` -- match domain against current rules ~1d #feat
- [ ] `bnvr query dns` -- resolve via Mihomo DNS engine ~1d #feat

## Phase 9: TUI Polish

- [ ] Live network speed graph (time-series plot) ~2d #tui
- [ ] Node card matrix with latency indicators ~2d #tui
- [ ] Real-time log stream panel from daemon ~1d #tui
- [ ] Keyboard shortcuts: node switching, panel focus ~1d #tui
- [ ] Status bar with active profile, kernel version, uptime ~1d #tui

---

### Notes

- Each phase should be **independently runnable** before moving to the next.
- Phase 1-2 are pure Rust learning (CLI, async, process management).
- Phase 3 introduces ratatui -- good TUI learning milestone.
- Phase 6 (Python bridge) is a core feature -- YAML processing is the main value-add.
- Phase 7 (TUN) is the hardest part; consider deferring until core is stable.
- Platforms: Windows + Linux only. No macOS.
