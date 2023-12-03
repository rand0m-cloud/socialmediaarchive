/// Core client interface
pub mod api;
/// Top-level client for logical operations
pub mod client;
/// REST interface to client
pub mod daemon;
/// File download client
pub mod download;
/// Description embedding client
pub mod embeddings;
/// File storage client
pub mod storage;
/// Vector database client
pub mod vector;

pub use api::ClientApi;
pub use client::{LocalClient, RemoteClient};
