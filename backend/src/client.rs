use crate::{
    daemon, download::DownloadClient, embeddings::EmbeddingClient, storage::StorageClient,
    vector::VectorDbClient,
};
use anyhow::*;
use serde::Serialize;
use serde_json::json;
use tempdir::TempDir;

/// A client that encapsulates all required components. Clones are referenced counted.
#[non_exhaustive]
#[derive(Clone)]
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

    /// Adds the requested link with description to the database.
    ///
    /// Downloads the file, stores the file, generates embeddings for the description,
    /// submits to the vector database.
    pub async fn add_link(&self, link: &str, description: &str) -> Result<Entry> {
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
        let payload = json!({"description": description, "original_link": link, "cid": cid.0});
        let id = self
            .vector
            .insert_vector(embeddings, payload.clone())
            .await?;

        Ok(Entry { id, payload })
    }

    /// Searches the vector database with the given description.
    ///
    /// Generates embeddings for the description and queries the vector database.
    pub async fn search(&self, description: &str) -> Result<SearchResult> {
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

    /// Runs the client as a daemon serving over REST
    pub async fn daemonize(self) -> Result<()> {
        Ok(daemon::run(self).await?)
    }
}

/// A collection of search entries.
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct SearchResult(pub Vec<SearchEntry>);

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self.0).unwrap())
    }
}

/// A search entry returned by the vector database.
#[derive(Debug, Serialize)]
pub struct SearchEntry {
    pub score: f32,
    #[serde(flatten)]
    pub entry: Entry,
}

/// A entry in the vector database.
#[derive(Debug, Serialize)]
pub struct Entry {
    pub id: String,
    pub payload: serde_json::Value,
}
