use crate::error::{JcvmError, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .unwrap(),
        }
    }

    /// Download a file with progress indication
    pub async fn download_with_progress<P: AsRef<Path>>(&self, url: &str, dest: P) -> Result<()> {
        let response =
            self.client
                .get(url)
                .send()
                .await
                .map_err(|e| JcvmError::DownloadFailed {
                    url: url.to_string(),
                    source: e,
                })?;

        let total_size = response.content_length().unwrap_or(0);

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(format!(
            "Downloading {}",
            url.split('/').last().unwrap_or("file")
        ));

        let mut file = File::create(dest.as_ref()).await?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| JcvmError::DownloadFailed {
                url: url.to_string(),
                source: e,
            })?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message("Download complete");
        Ok(())
    }

    /// Verify file checksum
    pub async fn verify_checksum<P: AsRef<Path>>(path: P, expected_checksum: &str) -> Result<bool> {
        let mut file = tokio::fs::File::open(path.as_ref()).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0; 8192];

        use tokio::io::AsyncReadExt;
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        let computed = format!("{:x}", result);

        Ok(computed.eq_ignore_ascii_case(expected_checksum))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_verify_checksum() {
        let temp_file = NamedTempFile::new().unwrap();
        tokio::fs::write(temp_file.path(), b"hello world")
            .await
            .unwrap();

        // SHA256 of "hello world"
        let checksum = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

        let result = Downloader::verify_checksum(temp_file.path(), checksum)
            .await
            .unwrap();
        assert!(result);
    }
}
