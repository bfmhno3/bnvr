mod cli;

use std::time::{SystemTime, UNIX_EPOCH};

use bnvr::daemon;
use bnvr::kernel;
use bnvr::network;
use bnvr::overwrite;
use bnvr::profile;
use clap::Parser;
use cli::{
    Cli, Commands, DaemonAction, KernelAction, NetworkAction, OverwriteAction, ProfileAction,
    QueryAction, StatsAction, TunAction,
};
use tracing::info;

#[tokio::main]
async fn main() {
    daemon::tracing_init::init();

    let cli = Cli::parse();
    info!("bnvr starting");

    match cli.command {
        None => {
            if let Err(e) = bnvr::tui::run().await {
                eprintln!("TUI error: {e}");
                std::process::exit(1);
            }
        }
        Some(cmd) => match cmd {
            Commands::Daemon { action } => match action {
                DaemonAction::Start => {
                    if let Err(e) = daemon::start::run().await {
                        eprintln!("daemon start failed: {e}");
                        std::process::exit(1);
                    }
                }
                DaemonAction::Stop => {
                    if let Err(e) = daemon::stop::run().await {
                        eprintln!("daemon stop failed: {e}");
                        std::process::exit(1);
                    }
                }
                DaemonAction::Status => {
                    if let Err(e) = daemon::status::run().await {
                        eprintln!("daemon status failed: {e}");
                        std::process::exit(1);
                    }
                }
            },
            Commands::Kernel { action } => match action {
                KernelAction::List => {
                    let installed = kernel::manage::list_installed();
                    if installed.is_empty() {
                        println!("no kernels installed");
                    } else {
                        for k in &installed {
                            let marker = if k.active { " *" } else { "" };
                            let status = if k.binary_exists {
                                "ok"
                            } else {
                                "missing binary"
                            };
                            println!("  {} [{}]{}", k.version, status, marker);
                        }
                    }
                }
                KernelAction::Install { version } => {
                    let ver = version.as_deref().unwrap_or("latest");
                    match kernel::download::download_and_extract(ver).await {
                        Ok(path) => println!("done: {}", path.display()),
                        Err(e) => {
                            eprintln!("install failed: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                KernelAction::Use { version } => {
                    if let Err(e) = kernel::manage::set_active(&version) {
                        eprintln!("failed: {e}");
                        std::process::exit(1);
                    }
                    println!("active kernel set to {}", version);
                }
                KernelAction::Status => {
                    let s = kernel::manage::kernel_status();
                    match &s.active_version {
                        Some(v) => {
                            println!("active: {}", v);
                            if s.binary_exists {
                                println!("binary: {}", s.binary_path.unwrap().display());
                            } else {
                                println!("binary: missing");
                            }
                        }
                        None => println!("no active kernel"),
                    }
                    match s.pid {
                        Some(pid) => println!("running: pid {}", pid),
                        None => println!("running: not running"),
                    }
                }
            },
            Commands::Profile { action } => {
                if let Err(e) = bnvr::paths::ensure_dirs() {
                    eprintln!("failed to create directories: {e}");
                    std::process::exit(1);
                }
                match action {
                    ProfileAction::Add {
                        url,
                        name,
                        user_agent,
                        auto_sync,
                        timeout,
                    } => {
                        if let Err(e) = profile::crud::add(
                            &name,
                            &url,
                            user_agent.as_deref(),
                            auto_sync.as_deref(),
                            timeout.as_deref(),
                        ) {
                            eprintln!("failed to add profile: {e}");
                            std::process::exit(1);
                        }
                        println!("profile '{}' added", name);
                    }
                    ProfileAction::Del { name } => {
                        if let Err(e) = profile::crud::del(&name) {
                            eprintln!("failed to delete profile: {e}");
                            std::process::exit(1);
                        }
                        println!("profile '{}' deleted", name);
                    }
                    ProfileAction::List => match profile::crud::list() {
                        Ok(profiles) => {
                            if profiles.is_empty() {
                                println!("no profiles configured");
                            } else {
                                for p in &profiles {
                                    let kind = match p.meta.kind {
                                        profile::crud::ProfileKind::Remote => "remote",
                                        profile::crud::ProfileKind::Merge => "merge",
                                    };
                                    let state = if p.has_raw { "synced" } else { "not synced" };
                                    let marker = if p.active { " *" } else { "" };
                                    let source = p
                                        .meta
                                        .url
                                        .as_deref()
                                        .map(str::to_string)
                                        .unwrap_or_else(|| p.meta.sources.join(" + "));
                                    let updated = p
                                        .meta
                                        .updated_at
                                        .map(|secs| format!(" (synced {} ago)", age(secs)))
                                        .unwrap_or_default();
                                    println!(
                                        "  {} [{}] [{}]{} {}{}",
                                        p.name, kind, state, marker, source, updated
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("failed to list profiles: {e}");
                            std::process::exit(1);
                        }
                    },
                    ProfileAction::Sync { name } => match name {
                        Some(n) => match profile::sync::sync_one(&n).await {
                            Ok(r) => println!("synced '{}': {} bytes", r.name, r.bytes),
                            Err(e) => {
                                eprintln!("sync failed: {e}");
                                std::process::exit(1);
                            }
                        },
                        None => match profile::sync::sync_all().await {
                            Ok(results) => {
                                if results.synced.is_empty() && results.failed.is_empty() {
                                    println!("no profiles to sync");
                                } else {
                                    for r in &results.synced {
                                        println!("synced '{}': {} bytes", r.name, r.bytes);
                                    }
                                    for failure in &results.failed {
                                        eprintln!("failed '{}': {}", failure.name, failure.error);
                                    }
                                    if !results.failed.is_empty() {
                                        std::process::exit(1);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("sync failed: {e}");
                                std::process::exit(1);
                            }
                        },
                    },
                    ProfileAction::Use { name } => match profile::crud::activate(&name).await {
                        Ok(path) => println!("active profile set to {name} -> {}", path.display()),
                        Err(e) => {
                            eprintln!("failed: {e}");
                            std::process::exit(1);
                        }
                    },
                    ProfileAction::View { path, name } => {
                        if let Err(e) = profile::view::view(name.as_deref(), path.as_deref()) {
                            eprintln!("view failed: {e}");
                            std::process::exit(1);
                        }
                    }
                    ProfileAction::Diff { name } => {
                        if let Err(e) = profile::diff::diff(name.as_deref()) {
                            eprintln!("diff failed: {e}");
                            std::process::exit(1);
                        }
                    }
                    ProfileAction::Merge { profiles, out } => {
                        match profile::merge::merge(&profiles, out.as_deref()) {
                            Ok(r) => {
                                let s = r.stats;
                                println!(
                                    "merged {} profiles into '{}': {} proxies ({} duplicates dropped), {} groups, {} rules",
                                    profiles.len(),
                                    r.name,
                                    s.proxies,
                                    s.dropped,
                                    s.groups,
                                    s.rules
                                );
                            }
                            Err(e) => {
                                eprintln!("merge failed: {e}");
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
            Commands::Overwrite { action } => {
                if let Err(e) = bnvr::paths::ensure_dirs() {
                    eprintln!("failed to create directories: {e}");
                    std::process::exit(1);
                }
                match action {
                    OverwriteAction::Add {
                        username,
                        link,
                        kind,
                        auto_sync,
                        timeout,
                    } => {
                        let parsed_kind = match kind.to_ascii_lowercase().as_str() {
                            "remote" => overwrite::crud::PluginKind::Remote,
                            "local" => overwrite::crud::PluginKind::Local,
                            _ => {
                                eprintln!("invalid plugin kind: {kind}");
                                std::process::exit(1);
                            }
                        };
                        match overwrite::crud::add(
                            &username,
                            &link,
                            parsed_kind,
                            auto_sync.as_deref(),
                            timeout.as_deref(),
                        ) {
                            Ok(dir) => println!("plugin '{}' added at {}", username, dir.display()),
                            Err(e) => {
                                eprintln!("add failed: {e}");
                                std::process::exit(1);
                            }
                        }
                    }
                    OverwriteAction::Init { name } => match overwrite::crud::init(&name) {
                        Ok(dir) => println!("plugin '{}' created at {}", name, dir.display()),
                        Err(e) => {
                            eprintln!("init failed: {e}");
                            std::process::exit(1);
                        }
                    },
                    OverwriteAction::List => match overwrite::crud::list() {
                        Ok(plugins) => {
                            if plugins.is_empty() {
                                println!("no plugins installed");
                            } else {
                                for p in &plugins {
                                    let marker = if p.active { " *" } else { "" };
                                    let kind = match p.kind {
                                        overwrite::crud::PluginKind::Remote => "remote",
                                        overwrite::crud::PluginKind::Local => "local",
                                    };
                                    let state = if p.has_entry {
                                        "ready"
                                    } else {
                                        "missing entry"
                                    };
                                    let venv = if p.has_venv { "ok" } else { "no venv" };
                                    println!(
                                        "  {} [{}] [{}]{} {}",
                                        p.username, kind, state, marker, venv
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("list failed: {e}");
                            std::process::exit(1);
                        }
                    },
                    OverwriteAction::Update { username } => {
                        if let Err(e) = overwrite::crud::update(&username) {
                            eprintln!("update failed: {e}");
                            std::process::exit(1);
                        }
                        println!("plugin '{}' updated", username);
                    }
                    OverwriteAction::Remove { username } => {
                        if let Err(e) = overwrite::crud::remove(&username) {
                            eprintln!("remove failed: {e}");
                            std::process::exit(1);
                        }
                        println!("plugin '{}' removed", username);
                    }
                    OverwriteAction::Use { name } => {
                        if let Err(e) = overwrite::crud::set_active(&name) {
                            eprintln!("failed: {e}");
                            std::process::exit(1);
                        }
                        println!("active plugin set to {}", name);
                    }
                    OverwriteAction::Git { args } => match overwrite::git::run_git_active(&args) {
                        Ok(output) => print!("{}", output),
                        Err(e) => {
                            eprintln!("git failed: {e}");
                            std::process::exit(1);
                        }
                    },
                }
            }
            Commands::Network { action } => match action {
                NetworkAction::Tun { action } => match action {
                    TunAction::Setup => {
                        if let Err(e) = network::tun::setup_tun().await {
                            eprintln!("network tun setup failed: {e}");
                            std::process::exit(1);
                        }
                    }
                    TunAction::Clear => {
                        if let Err(e) = network::tun::clear_tun().await {
                            eprintln!("network tun clear failed: {e}");
                            std::process::exit(1);
                        }
                    }
                },
                NetworkAction::Bypass { target } => {
                    if let Err(e) = network::bypass::add_bypass_route(&target).await {
                        eprintln!("network bypass failed: {e}");
                        std::process::exit(1);
                    }
                }
            },
            Commands::Bench { group } => {
                println!("bench {}", group.unwrap_or_default())
            }
            Commands::Stats { action } => match action {
                StatsAction::Top => println!("stats top"),
                StatsAction::Summary => println!("stats summary"),
                StatsAction::Nodes => println!("stats nodes"),
            },
            Commands::Query { action } => match action {
                QueryAction::Rule { target } => println!("query rule {}", target),
                QueryAction::Dns => println!("query dns"),
            },
            Commands::Tldr => print_tldr(),
        },
    }
}

fn age(secs: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(secs);
    let elapsed = now.saturating_sub(secs);
    match elapsed {
        0..=59 => format!("{elapsed}s"),
        60..=3599 => format!("{}m", elapsed / 60),
        3600..=86399 => format!("{}h", elapsed / 3600),
        _ => format!("{}d", elapsed / 86400),
    }
}

fn print_tldr() {
    println!(
        r#"bnvr tldr              Quick reference
bnvr                   Launch TUI
bnvr daemon start      Start background daemon
bnvr daemon stop       Stop daemon
bnvr daemon status     Daemon health check
bnvr kernel list       List kernel versions
bnvr kernel install    Download Mihomo kernel
bnvr kernel use <ver>  Switch kernel version
bnvr profile list      List subscriptions
bnvr profile add       Add subscription
bnvr profile add <n> <u> --auto-sync 1d  Add daily auto-sync
bnvr profile sync      Fetch & process config
bnvr profile use <n>   Activate profile
bnvr profile merge a b Merge subscriptions
bnvr overwrite list    List Python plugins
bnvr overwrite add <u> <link> Clone plugin from Git
bnvr overwrite update <u>     Pull latest plugin changes
bnvr overwrite init    Create new plugin
bnvr network tun setup Take over routing
bnvr bench [group]     Run network benchmarks
bnvr stats top         Top domains by traffic
bnvr query rule <dom>  Query rule match"#
    );
}
