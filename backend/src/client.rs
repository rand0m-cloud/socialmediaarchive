use std::{sync::Arc, time::Duration};

use crate::{
    api::*,
    daemon::{self, AddLink, Task},
    download::DownloadClient,
    embeddings::EmbeddingClient,
    storage::StorageClient,
    vector::VectorDbClient,
};
use anyhow::*;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::{from_value, json};
use tempdir::TempDir;

/// A top-level client that encapsulates all required components and provides the logical operations.
/// Clones are referenced counted.
#[non_exhaustive]
#[derive(Clone)]
pub struct LocalClient {
    pub embeddings: EmbeddingClient,
    pub vector: VectorDbClient,
    pub storage: StorageClient,
    pub download: DownloadClient,
}

impl LocalClient {
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

    /// Runs the client as a daemon serving over REST
    pub async fn daemonize(self) -> Result<()> {
        Ok(daemon::run(self).await?)
    }
}

#[async_trait(?Send)]
impl ClientApi for LocalClient {
    async fn add_link(&self, link: &str, description: &str) -> Result<Entry> {
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

    async fn search(&self, description: &str) -> Result<SearchResult> {
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

/// Similiar to a LocalClient but for daemons that are remote.
/// Clones are referenced counted.
#[derive(Clone)]
pub struct RemoteClient {
    web_client: reqwest::Client,
    url: Arc<str>,
}

impl RemoteClient {
    pub fn new(url: &str) -> Self {
        Self {
            web_client: reqwest::Client::new(),
            url: Arc::from(url),
        }
    }

    /// Waits for the given task to complete and deserializes the result
    async fn wait_for_task<T: DeserializeOwned>(&self, task: &str) -> Result<T> {
        loop {
            let resp = self
                .web_client
                .get(format!("{}{task}", self.url))
                .send()
                .await
                .with_context(|| format!("failed to use API endpoint {task}"))?;
            let task: Task = resp.json().await?;
            match task {
                Task::Cancelled => bail!("task was cancelled"),
                Task::InProgress { .. } => tokio::time::sleep(Duration::from_secs(1)).await,
                Task::Completed { data } => return Ok(from_value(data)?),
            }
        }
    }
}

#[async_trait(?Send)]
impl ClientApi for RemoteClient {
    async fn add_link(&self, link: &str, description: &str) -> Result<Entry> {
        let resp = self
            .web_client
            .post(format!("{}/api/v0/add", self.url))
            .json(&AddLink {
                link: link.to_string(),
                description: description.to_string(),
            })
            .send()
            .await
            .context("failed to use API endpoint /add")?;
        let task = resp.headers().get("location").unwrap().to_str()?;
        self.wait_for_task(task).await
    }

    async fn search(&self, description: &str) -> Result<SearchResult> {
        let resp = self
            .web_client
            .post(format!("{}/api/v0/search", self.url))
            .body(description.to_string())
            .send()
            .await
            .context("failed to use API endpoint /search")?;
        let task = resp.headers().get("location").unwrap().to_str()?;
        self.wait_for_task(task).await
    }
}
