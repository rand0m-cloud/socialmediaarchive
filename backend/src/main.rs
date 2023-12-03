use std::io::read_to_string;

use anyhow::{Context, Result};
use backend::{api::*, client::RemoteClient, LocalClient};
use clap::*;
use tracing::warn;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a social media link to the archive
    Add { link: String },
    /// Search the archive for description
    Search {},
    /// Runs a daemon that provides a HTTP REST interface
    Daemon {},
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    if dotenv::dotenv().is_err() {
        warn!("didn't load a .env file")
    }

    let args = Cli::parse();

    let client: Box<dyn ClientApi> = if let Ok(url) = std::env::var("API_URL") {
        Box::new(RemoteClient::new(&url))
    } else {
        Box::new(
            LocalClient::new()
                .await
                .context("failed to create client")?,
        )
    };

    match args.command {
        Commands::Add { link } => {
            println!("Enter description for this post:");
            let input = read_to_string(std::io::stdin())?;

            client.add_link(&link, &input).await?;
        }
        Commands::Search {} => {
            println!("Enter description to search by:");
            let input = read_to_string(std::io::stdin())?;

            let results = client.search(&input).await?;
            println!("{results}");
        }
        Commands::Daemon {} => {
            LocalClient::new()
                .await
                .context("failed to create local client")?
                .daemonize()
                .await?;
        }
    }

    Ok(())
}
