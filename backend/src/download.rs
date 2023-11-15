use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::*;

#[derive(Debug)]
pub struct DownloadClient {}

impl DownloadClient {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub async fn download(&mut self, url: &str, dir: impl AsRef<Path>) -> Result<PathBuf> {
        let dir = dir.as_ref();

        if std::fs::read_dir(dir)?.count() != 0 {
            bail!("download client was passed an non-empty directory");
        }

        let exit = Command::new("yt-dlp")
            .args(["--add-header", "accept:*/*", "--no-playlist", url])
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .current_dir(dir)
            .output()?
            .status;

        if !exit.success() {
            bail!("yt-dlp command failed with {exit}");
        }

        let file_count = std::fs::read_dir(dir)?.count();
        if file_count != 1 {
            bail!("yt-dlp created {file_count} files instead of one");
        }

        let outfile = std::fs::read_dir(dir)?.next().unwrap()?.path();

        Ok(outfile)
    }
}
