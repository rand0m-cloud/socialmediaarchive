use anyhow::*;
use download::DownloadClient;
use embeddings::EmbeddingClient;
use storage::StorageClient;
use vector::VectorDbClient;

mod download;
mod embeddings;
mod storage;
mod vector;

#[non_exhaustive]
pub struct Client {
    pub embeddings: EmbeddingClient,
    pub vector: VectorDbClient,
    pub storage: StorageClient,
    pub download: DownloadClient,
}

impl Client {
    pub async fn new() -> Result<Self> {
        let embeddings = EmbeddingClient::new().context("failed to create embeddings client")?;
        let mut vector = VectorDbClient::new().context("failed to create vectordb client")?;
        let download = DownloadClient::new().context("failed to create download client")?;
        let mut storage = StorageClient::new().context("failed to create storage client")?;

        vector
            .init()
            .await
            .context("failed to initialize vectordb")?;
        storage
            .init()
            .await
            .context("failed to initialize storage")?;

        Ok(Self {
            embeddings,
            vector,
            storage,
            download,
        })
    }
}
