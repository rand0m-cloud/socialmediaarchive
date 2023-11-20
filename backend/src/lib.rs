use anyhow::*;
use download::DownloadClient;
use embeddings::EmbeddingClient;
use serde::Serialize;
use serde_json::json;
use storage::StorageClient;
use tempdir::TempDir;
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

    pub async fn add_link(&mut self, link: &str, description: &str) -> Result<()> {
        let temp = TempDir::new("socialmediadownload")?;

        // download the requested link
        let outfile = self
            .download
            .download(link, temp.path())
            .await
            .context("failed to download link")?;

        let cid = self
            .storage
            .save_file(outfile)
            .await
            .context("failed to store downloaded file")?;

        // generate embeddings and store in vector db
        let embeddings = self.embeddings.generate(description.trim()).await?;
        self.vector
            .insert_vector(
                embeddings,
                json!({"description": description, "original_link": link, "cid": cid.0}),
            )
            .await?;

        Ok(())
    }

    pub async fn search(&mut self, description: &str) -> Result<SearchResult> {
        let embedding = self
            .embeddings
            .generate(description.trim())
            .await
            .context("failed to generate embedding for description")?;
        self.vector
            .search(embedding)
            .await
            .context("failed to search vector db")
    }
}

#[derive(Debug)]
pub struct SearchResult(pub Vec<Entry>);

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self.0).unwrap())
    }
}

#[derive(Debug, Serialize)]
pub struct Entry {
    id: String,
    score: f32,
    payload: serde_json::Value,
}
