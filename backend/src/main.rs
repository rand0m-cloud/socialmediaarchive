use anyhow::{Context, Result};
use backend::Client;
use clap::*;
use serde_json::json;
use tempdir::TempDir;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().context("missing .env file")?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Cli::parse();

    let mut client = Client::new().await.context("failed to create client")?;

    match args.command {
        Commands::Add { link } => {
            let temp = TempDir::new("socialmediadownload")?;

            // download the requested link
            let outfile = client
                .download
                .download(&link, temp.path())
                .await
                .context("failed to download link")?;

            let cid = client
                .storage
                .save_file(outfile)
                .await
                .context("failed to store downloaded file")?;

            // ask for user description
            println!("Enter description for this post:");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input = input.trim().to_string();

            // generate embeddings and store in vector db
            let embeddings = client.embeddings.generate(&input).await?;
            client
                .vector
                .insert_vector(
                    embeddings,
                    json!({"description": input, "original_link": link, "cid": cid.0}),
                )
                .await?;
        }
    }

    Ok(())
}
