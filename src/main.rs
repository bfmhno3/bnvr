mod cli;

use bnvr::daemon;
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
            println!("Launching TUI (not implemented yet)");
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
                    if let Err(e) = daemon::stop::run() {
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
                KernelAction::List => println!("kernel list"),
                KernelAction::Install => println!("kernel install"),
                KernelAction::Use { version } => println!("kernel use {}", version),
                KernelAction::Status => println!("kernel status"),
            },
            Commands::Profile { action } => match action {
                ProfileAction::Add { url, name } => println!("profile add {} {}", url, name),
                ProfileAction::Del { name } => println!("profile del {}", name),
                ProfileAction::List => println!("profile list"),
                ProfileAction::Sync { name } => {
                    println!("profile sync {}", name.unwrap_or_default())
                }
                ProfileAction::View { path } => {
                    println!("profile view {}", path.unwrap_or_default())
                }
                ProfileAction::Diff => println!("profile diff"),
                ProfileAction::Merge { profiles, out } => {
                    println!("profile merge {:?} {:?}", profiles, out)
                }
            },
            Commands::Overwrite { action } => match action {
                OverwriteAction::Init { name } => println!("overwrite init {}", name),
                OverwriteAction::List => println!("overwrite list"),
                OverwriteAction::Use { name } => println!("overwrite use {}", name),
                OverwriteAction::Git { args } => println!("overwrite git {:?}", args),
            },
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
