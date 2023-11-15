use std::path::Path;

use anyhow::*;
use ipfs_api::{IpfsApi, IpfsClient, TryFromUri};
use tracing::warn;

pub struct StorageClient {
    ipfs: IpfsClient,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Cid(pub String);

impl std::fmt::Display for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl StorageClient {
    pub fn new() -> Result<Self> {
        let ipfs =
            IpfsClient::from_str(&std::env::var("IPFS_URL").context("IPFS_URL env var not set")?)?;
        Ok(Self { ipfs })
    }

    pub async fn init(&mut self) -> Result<()> {
        let _ = self.ipfs.files_mkdir("/socialmediaarchive", false).await;
        Ok(())
    }

    pub async fn save_file(&mut self, filepath: impl AsRef<Path>) -> Result<Cid> {
        let filepath = filepath.as_ref();
        let downloaded_file = std::fs::File::open(filepath)
            .context("failed to open downloaded file to send to ipfs")?;
        let res = self
            .ipfs
            .add(downloaded_file)
            .await
            .context("failed to add file to ipfs")?;
        let cid = res.hash;
        self.ipfs
            .pin_add(&cid, true)
            .await
            .context("failed to pin content to ipfs")?;
        if let Err(e) = self.ipfs
            .files_cp(
                &format!("/ipfs/{cid}"),
                &format!(
                    "/socialmediaarchive/{}",
                    filepath.file_name().unwrap().to_str().unwrap()
                ),
            )
            .await
        {
            warn!("couldn't save content to ipfs files (maybe filename already exists?): {e}");
        }

        Ok(Cid(cid))
    }
}
