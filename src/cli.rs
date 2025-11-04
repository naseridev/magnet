use clap::{Parser, Subcommand};
use std::path::PathBuf;

const VERSION: &str = "3.0.0";

#[derive(Parser)]
#[command(name = "magnet")]
#[command(version = VERSION)]
#[command(about = "Enterprise-grade GitHub package manager for installing and managing GitHub releases", long_about = None)]
#[command(styles = get_styles())]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(
        long,
        env = "GITHUB_TOKEN",
        hide_env_values = true,
        help = "GitHub personal access token for authenticated API requests (increases rate limits)"
    )]
    pub token: Option<String>,

    #[arg(
        long,
        short = 'v',
        global = true,
        help = "Enable verbose output with detailed progress information"
    )]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Install a package binary from GitHub releases")]
    #[command(
        long_about = "Downloads and installs the latest release binary for your platform. Automatically detects OS and architecture to select the correct asset."
    )]
    Install {
        #[arg(help = "Package identifier in format: owner/repo (e.g., BurntSushi/ripgrep)")]
        package: String,

        #[arg(
            long,
            short = 'V',
            help = "Specific version tag to install (e.g., v1.2.3). Defaults to latest release if not specified"
        )]
        version: Option<String>,

        #[arg(
            long,
            short = 'g',
            help = "Install globally to system-wide location (/usr/local/bin on Unix, Program Files on Windows)"
        )]
        global: bool,

        #[arg(
            long,
            short = 'f',
            help = "Force reinstallation even if the package is already installed"
        )]
        force: bool,
    },

    #[command(about = "Remove an installed package and its binaries")]
    #[command(
        long_about = "Uninstalls the package by removing all associated binaries from the installation directory and updating the registry."
    )]
    Uninstall {
        #[arg(help = "Package identifier in format: owner/repo")]
        package: String,

        #[arg(
            long,
            short = 'g',
            help = "Uninstall from global installation directory"
        )]
        global: bool,
    },

    #[command(about = "List all installed packages")]
    #[command(
        long_about = "Display all packages installed locally and globally with their versions, installation dates, and binary locations."
    )]
    List {
        #[arg(
            long,
            short = 'v',
            help = "Show detailed information including installation paths, binaries, and timestamps"
        )]
        verbose: bool,
    },

    #[command(about = "Update installed packages to their latest versions")]
    #[command(
        long_about = "Check for and install newer versions of packages. Updates a specific package if provided, otherwise updates all installed packages."
    )]
    Update {
        #[arg(help = "Package identifier in format: owner/repo (updates all packages if omitted)")]
        package: Option<String>,

        #[arg(
            long,
            short = 'g',
            help = "Update packages in global installation directory"
        )]
        global: bool,
    },

    #[command(about = "Display detailed repository and release information")]
    #[command(
        long_about = "Fetch and display comprehensive information about a GitHub repository including description, stars, language, and latest release details."
    )]
    Info {
        #[arg(help = "Package identifier in format: owner/repo")]
        package: String,
    },

    #[command(about = "Search for packages and users on GitHub")]
    #[command(
        long_about = "Search GitHub repositories or users using GitHub's search API. Returns matching repositories with descriptions, stars, and language information."
    )]
    Search {
        #[arg(help = "Search query - can be a username, owner/repo, or search term")]
        query: String,

        #[arg(
            long,
            short = 'l',
            default_value = "10",
            help = "Maximum number of search results to return (1-100)"
        )]
        limit: usize,

        #[arg(
            long,
            short = 'u',
            help = "Search for GitHub users instead of repositories"
        )]
        by_user: bool,
    },

    #[command(about = "Clean up untracked files from installation directories")]
    #[command(
        long_about = "Remove binaries that exist in the installation directory but are not registered in the package registry. Useful for cleaning up failed installations."
    )]
    Clean {
        #[arg(long, short = 'g', help = "Clean global installation directory")]
        global: bool,
    },

    #[command(about = "Bulk download all repositories from a GitHub user")]
    #[command(
        long_about = "Clone or download all repositories from a GitHub user with advanced filtering options. Supports parallel downloads and comprehensive filtering by language, stars, size, and more."
    )]
    Dump {
        #[arg(help = "GitHub username whose repositories will be downloaded")]
        username: String,

        #[arg(
            long,
            short = 'l',
            help = "Filter repositories by programming language (e.g., Rust, Python, JavaScript)"
        )]
        language: Option<String>,

        #[arg(long, short = 's', help = "Only include repositories with at least this many stars", value_parser = clap::value_parser!(u32))]
        min_stars: Option<u32>,

        #[arg(
            long,
            short = 'o',
            help = "Exclude forked repositories, only download original repositories"
        )]
        only_original: bool,

        #[arg(
            long,
            short = 'r',
            help = "Filter repository names using regex pattern (e.g., '^rust-.*' for repos starting with 'rust-')"
        )]
        regex: Option<String>,

        #[arg(long, short = 'm', help = "Skip repositories larger than this size in megabytes", value_parser = clap::value_parser!(u32))]
        max_size: Option<u32>,

        #[arg(long, short = 'p', help = "Number of concurrent downloads (higher = faster but more resource intensive)", value_parser = clap::value_parser!(usize), default_value = "3")]
        parallel: usize,

        #[arg(
            long,
            short = 'd',
            help = "Directory where repositories will be downloaded (defaults to username)"
        )]
        output: Option<PathBuf>,
    },
}

fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .header(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightBlue))),
        )
        .usage(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightBlue))),
        )
        .literal(
            anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
        .placeholder(
            anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
}
