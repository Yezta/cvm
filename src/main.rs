mod api;
mod cli;
mod config;
mod detect;
mod download;
mod error;
mod install;
mod models;
mod shell;
mod utils;
mod version_manager;

use anyhow::Result;
use cli::Cli;
use config::Config;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    // Load configuration
    let config = Config::load().map_err(|e| anyhow::anyhow!(e))?;

    // Parse CLI arguments and execute
    let cli = Cli::new(config);
    cli.run().await.map_err(|e| anyhow::anyhow!(e))
}
