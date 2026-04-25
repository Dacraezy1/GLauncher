use anyhow::{Result, Context};
use futures::StreamExt;
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub filename: String,
}

pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    progress_tx: Option<tokio::sync::mpsc::Sender<DownloadProgress>>,
) -> Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let resp = client.get(url).send().await
        .with_context(|| format!("Failed to GET {url}"))?;

    let total = resp.content_length().unwrap_or(0);
    let filename = dest.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut file = tokio::fs::File::create(dest).await?;
    let mut stream = resp.bytes_stream();
    let mut downloaded = 0u64;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if let Some(ref tx) = progress_tx {
            let _ = tx.send(DownloadProgress {
                downloaded,
                total,
                filename: filename.clone(),
            }).await;
        }
    }

    file.flush().await?;
    Ok(())
}

pub async fn download_file_if_missing(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<()> {
    if dest.exists() {
        return Ok(());
    }
    download_file(client, url, dest, None).await
}

pub fn build_http_client() -> Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .user_agent("GLauncher/1.0.0 (Linux; +https://github.com/Dacraezy1/GLauncher)")
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    Ok(client)
}
