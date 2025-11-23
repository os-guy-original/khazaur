mod cli;
mod config;
mod error;
mod aur;
mod pacman;
mod resolver;
mod build;
mod ui;
mod dirs;
mod flatpak;
mod snap;
mod debtap;
mod debian;
mod cache;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments first to check verbose flag
    let args = cli::Args::parse();

    // Initialize logging based on verbose flag
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(log_level.into())
        )
        .with_target(false)
        .init();

    // Execute the appropriate command
    cli::execute(args).await?;

    Ok(())
}
