use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "bnvr", version, about = "BNVR is Not Verge Rev")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Manage Mihomo kernel versions
    Kernel {
        #[command(subcommand)]
        action: KernelAction,
    },

    /// Manage subscription profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// Manage Python overwrite plugins
    Overwrite {
        #[command(subcommand)]
        action: OverwriteAction,
    },

    /// Network layer and TUN management
    Network {
        #[command(subcommand)]
        action: NetworkAction,
    },

    /// Run network benchmarks
    Bench {
        /// Optional group name to benchmark
        group: Option<String>,
    },

    /// View traffic statistics
    Stats {
        #[command(subcommand)]
        action: StatsAction,
    },

    /// Query rules and DNS
    Query {
        #[command(subcommand)]
        action: QueryAction,
    },

    /// Print a quick reference of common commands
    Tldr,
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

#[derive(Subcommand, Debug)]
pub enum KernelAction {
    /// List installed and available kernels
    List,
    /// Download and install a kernel version
    Install {
        /// Version to install (e.g. v1.19.27). Fetches latest if omitted.
        version: Option<String>,
    },
    /// Switch active kernel version
    Use {
        /// Version to activate
        version: String,
    },
    /// Show running kernel status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    /// Add a subscription source
    Add {
        /// Subscription URL
        url: String,
        /// Profile name
        name: String,
        /// Override the User-Agent sent when syncing this profile
        #[arg(long)]
        user_agent: Option<String>,
    },
    /// Remove a subscription source
    Del { name: String },
    /// List stored profiles
    List,
    /// Fetch and store a subscription
    Sync { name: Option<String> },
    /// Activate a profile and write BNVR_HOME/config.yaml
    Use { name: String },
    /// View config at a specific path
    View {
        /// YAML path to navigate (e.g. proxies.0)
        path: Option<String>,
        /// Profile name (defaults to the active profile)
        #[arg(long)]
        name: Option<String>,
    },
    /// Compare raw vs processed config
    Diff {
        /// Profile name (defaults to the active profile)
        #[arg(long)]
        name: Option<String>,
    },
    /// Merge multiple profiles
    Merge {
        /// Profile names to merge
        profiles: Vec<String>,
        /// Output profile name (defaults to the sources joined with '+')
        #[arg(long)]
        out: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum OverwriteAction {
    /// Create a new overwrite plugin with isolated venv
    Init {
        /// Plugin module name
        name: String,
    },
    /// List installed plugins
    List,
    /// Activate a plugin
    Use {
        /// Plugin name to activate
        name: String,
    },
    /// Run git commands in a plugin's directory
    Git {
        /// Arguments to pass to git
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum NetworkAction {
    /// TUN interface management
    Tun {
        #[command(subcommand)]
        action: TunAction,
    },
    /// Add a direct bypass route
    Bypass {
        /// IP or CIDR to bypass
        target: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TunAction {
    /// Set up TUN and take over routing
    Setup,
    /// Tear down TUN and restore routes
    Clear,
}

#[derive(Subcommand, Debug)]
pub enum StatsAction {
    /// Top domains by traffic
    Top,
    /// Traffic summary
    Summary,
    /// Per-node statistics
    Nodes,
}

#[derive(Subcommand, Debug)]
pub enum QueryAction {
    /// Query rule match for a domain or IP
    Rule {
        /// Domain or IP to query
        target: String,
    },
    /// Query DNS resolution
    Dns,
}
