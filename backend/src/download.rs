use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::*;

#[derive(Debug, Clone)]
pub struct DownloadClient {}

impl DownloadClient {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    pub async fn download(&self, url: &str, dir: impl AsRef<Path>) -> Result<PathBuf> {
        let dir = dir.as_ref();

        ensure!(
            std::fs::read_dir(dir)?.count() == 0,
            "download client was passed an non-empty directory"
        );

        let exit = Command::new("yt-dlp")
            .args(["--add-header", "accept:*/*", "--no-playlist", url])
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .current_dir(dir)
            .output()
            .context("failed to run yt-dlp command")?
            .status;

        ensure!(exit.success(), "yt-dlp command failed with {exit}");

        let file_count = std::fs::read_dir(dir)?.count();
        ensure!(
            file_count == 1,
            "yt-dlp created {file_count} files instead of one"
        );

        let outfile = std::fs::read_dir(dir)?.next().unwrap()?.path();

        Ok(outfile)
    }
}
