use anyhow::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A collection of search entries.
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SearchResult(pub Vec<SearchEntry>);

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self.0).unwrap())
    }
}

/// A search entry returned by the vector database.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEntry {
    pub score: f32,
    #[serde(flatten)]
    pub entry: Entry,
}

/// A entry in the vector database.
#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub payload: serde_json::Value,
}

/// The top-level API of this project
#[async_trait(?Send)]
pub trait ClientApi: Send + Sync + 'static {
    /// Adds the requested link with description to the database.
    ///
    /// Downloads the file, stores the file, generates embeddings for the description,
    /// submits to the vector database.
    async fn add_link(&self, link: &str, description: &str) -> Result<Entry>;

    /// Searches the vector database with the given description.
    ///
    /// Generates embeddings for the description and queries the vector database.
    async fn search(&self, description: &str) -> Result<SearchResult>;
}
