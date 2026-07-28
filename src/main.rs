mod cli;

use bnvr::daemon;
use bnvr::kernel;
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
                    if let Err(e) = daemon::status::run() {
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
                            let status = if k.binary_exists { "ok" } else { "missing binary" };
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
                    match s.active_version {
                        Some(ref v) => {
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
                let conn = match daemon::db::open() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("failed to open database: {e}");
                        std::process::exit(1);
                    }
                };
                match action {
                    ProfileAction::Add { url, name } => {
                        if let Err(e) = profile::crud::add(&conn, &name, &url) {
                            eprintln!("failed to add profile: {e}");
                            std::process::exit(1);
                        }
                        println!("profile '{}' added", name);
                    }
                    ProfileAction::Del { name } => {
                        if let Err(e) = profile::crud::del(&conn, &name) {
                            eprintln!("failed to delete profile: {e}");
                            std::process::exit(1);
                        }
                        println!("profile '{}' deleted", name);
                    }
                    ProfileAction::List => {
                        match profile::crud::list(&conn) {
                            Ok(profiles) => {
                                if profiles.is_empty() {
                                    println!("no profiles configured");
                                } else {
                                    for p in &profiles {
                                        let sync_status = if p.raw_config.is_some() { "synced" } else { "not synced" };
                                        println!("  {} [{}] {}", p.name, sync_status, p.url);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("failed to list profiles: {e}");
                                std::process::exit(1);
                            }
                        }
                    }
                    ProfileAction::Sync { name } => {
                        match name {
                            Some(n) => {
                                match profile::sync::sync_one(&conn, &n).await {
                                    Ok(r) => println!("synced '{}': {} bytes", r.name, r.bytes),
                                    Err(e) => {
                                        eprintln!("sync failed: {e}");
                                        std::process::exit(1);
                                    }
                                }
                            }
                            None => {
                                match profile::sync::sync_all(&conn).await {
                                    Ok(results) => {
                                        if results.is_empty() {
                                            println!("no profiles to sync");
                                        } else {
                                            for r in &results {
                                                println!("synced '{}': {} bytes", r.name, r.bytes);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("sync failed: {e}");
                                        std::process::exit(1);
                                    }
                                }
                            }
                        }
                    }
                    ProfileAction::View { path } => {
                        // Default to first profile if no name specified
                        let profiles = match profile::crud::list(&conn) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("failed to list profiles: {e}");
                                std::process::exit(1);
                            }
                        };
                        if profiles.is_empty() {
                            eprintln!("no profiles configured");
                            std::process::exit(1);
                        }
                        let name = &profiles[0].name;
                        if let Err(e) = profile::view::view(&conn, name, path.as_deref()) {
                            eprintln!("view failed: {e}");
                            std::process::exit(1);
                        }
                    }
                    ProfileAction::Diff => {
                        let profiles = match profile::crud::list(&conn) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("failed to list profiles: {e}");
                                std::process::exit(1);
                            }
                        };
                        if profiles.is_empty() {
                            eprintln!("no profiles configured");
                            std::process::exit(1);
                        }
                        let name = &profiles[0].name;
                        if let Err(e) = profile::diff::diff(&conn, name) {
                            eprintln!("diff failed: {e}");
                            std::process::exit(1);
                        }
                    }
                    ProfileAction::Merge { .. } => {
                        eprintln!("merge not yet implemented");
                        std::process::exit(1);
                    }
                }
            }
            Commands::Overwrite { action } => {
                if let Err(e) = bnvr::paths::ensure_dirs() {
                    eprintln!("failed to create directories: {e}");
                    std::process::exit(1);
                }
                match action {
                    OverwriteAction::Init { name } => {
                        match overwrite::crud::init(&name) {
                            Ok(dir) => println!("plugin '{}' created at {}", name, dir.display()),
                            Err(e) => {
                                eprintln!("init failed: {e}");
                                std::process::exit(1);
                            }
                        }
                    }
                    OverwriteAction::List => {
                        match overwrite::crud::list() {
                            Ok(plugins) => {
                                if plugins.is_empty() {
                                    println!("no plugins installed");
                                } else {
                                    for p in &plugins {
                                        let marker = if p.active { " *" } else { "" };
                                        let venv = if p.has_venv { "ok" } else { "no venv" };
                                        println!("  {} [{}]{}", p.name, venv, marker);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("list failed: {e}");
                                std::process::exit(1);
                            }
                        }
                    }
                    OverwriteAction::Use { name } => {
                        if let Err(e) = overwrite::crud::set_active(&name) {
                            eprintln!("failed: {e}");
                            std::process::exit(1);
                        }
                        println!("active plugin set to {}", name);
                    }
                    OverwriteAction::Git { args } => {
                        match overwrite::git::run_git_active(&args) {
                            Ok(output) => print!("{}", output),
                            Err(e) => {
                                eprintln!("git failed: {e}");
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
            Commands::Network { action } => match action {
                NetworkAction::Tun { action } => match action {
                    TunAction::Setup => println!("network tun setup"),
                    TunAction::Clear => println!("network tun clear"),
                },
                NetworkAction::Bypass { target } => println!("network bypass {}", target),
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
bnvr profile sync      Fetch & process config
bnvr overwrite list    List Python plugins
bnvr overwrite init    Create new plugin
bnvr network tun setup Take over routing
bnvr bench [group]     Run network benchmarks
bnvr stats top         Top domains by traffic
bnvr query rule <dom>  Query rule match"#
    );
}
