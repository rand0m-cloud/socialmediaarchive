use anyhow::{ Context, Result, ensure};
use reqwest::Client;
use serde_json::{from_value, json, Value};
use tracing::instrument;

#[derive(Debug, Clone)]
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
    pub async fn generate(&self, input: &str) -> Result<Vec<f32>> {
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

        ensure!(
            resp.get("error").is_none(),
            "OpenAi embeddings request failed: {}",
            resp["error"]
        );

        Ok(from_value(resp["data"][0]["embedding"].take())?)
    }
}
