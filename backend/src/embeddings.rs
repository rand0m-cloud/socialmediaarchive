use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::{from_value, json, Value};
use tracing::instrument;

#[derive(Debug)]
pub struct EmbeddingClient {
    key: String,
    client: Client,
}

impl EmbeddingClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            key: std::env::var("OPENAI_KEY").context("OPENAI_KEY env variable not set")?,
            client: Client::new(),
        })
    }

    #[instrument(skip_all)]
    pub async fn generate(&mut self, input: &str) -> Result<Vec<f32>> {
        let mut resp: Value = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.key)
            .json(&json!({
                "model": "text-embedding-ada-002",
                "input": input
            }))
            .send()
            .await
            .context("failed to send api request to OpenAi for embeddings")?
            .json()
            .await?;

        if resp.get("error").is_some() {
            bail!("OpenAi embeddings request failed: {}", resp["error"]);
        }

        Ok(from_value(resp["data"][0]["embedding"].take())?)
    }
}
