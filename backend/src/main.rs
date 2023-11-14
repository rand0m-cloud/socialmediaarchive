use anyhow::{Context, Result};
use backend::Client;
use serde_json::json;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;

    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let mut client = Client::new().await.context("failed to create client")?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    input = input.trim().to_string();

    let embeddings = client.embeddings.generate(&input).await?;
    client.vector.insert_vector(embeddings, json!({"input": input})).await?;

    Ok(())
}
