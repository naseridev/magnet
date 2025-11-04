use anyhow::Result;
use clap::Parser;

mod archive;
mod cli;
mod colors;
mod errors;
mod github;
mod install;
mod manager;
mod registry;
mod types;
mod utils;

use cli::Cli;
use manager::PackageManager;

#[tokio::main]
async fn main() -> Result<()> {
    colors::init_colors();

    let cli = Cli::parse();
    let manager = PackageManager::new(cli.token.clone(), cli.verbose)?;
    manager.execute(cli.command).await
}
