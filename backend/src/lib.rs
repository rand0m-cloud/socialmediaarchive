use anyhow::*;
use embeddings::EmbeddingClient;
use vector::VectorDbClient;

mod embeddings;
mod vector;

#[non_exhaustive]
pub struct Client {
    pub embeddings: EmbeddingClient,
    pub vector: VectorDbClient,
}

impl Client {
    pub async fn new() -> Result<Self> {
        let embeddings =
            embeddings::EmbeddingClient::new().context("failed to create embeddings client")?;
        let mut vector =
            vector::VectorDbClient::new().context("failed to create vectordb client")?;

        vector
            .init()
            .await
            .context("failed to initialize vectordb")?;

        Ok(Self { embeddings, vector })
    }
}
