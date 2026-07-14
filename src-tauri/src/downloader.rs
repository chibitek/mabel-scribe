use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

pub async fn download_model(
    app: AppHandle,
    url: &str,
    dest: &PathBuf,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total = response.content_length().unwrap_or(0);

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Write to a sibling .part file, then rename. Large-v3 Q5 is ~1.1 GB;
    // an interrupted write must not leave a truncated dest that later
    // looks "already downloaded".
    let partial = dest.with_extension("bin.part");
    let mut file = std::fs::File::create(&partial).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => {
                let _ = std::fs::remove_file(&partial);
                return Err(format!("Download stream error: {}", error));
            }
        };
        if let Err(error) = file.write_all(&chunk) {
            let _ = std::fs::remove_file(&partial);
            return Err(error.to_string());
        }
        downloaded += chunk.len() as u64;

        let percent = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                downloaded,
                total,
                percent,
            },
        );
    }

    file.sync_all().map_err(|error| {
        let _ = std::fs::remove_file(&partial);
        error.to_string()
    })?;

    if total > 0 && downloaded != total {
        let _ = std::fs::remove_file(&partial);
        return Err(format!(
            "Download incomplete: received {} of {} bytes",
            downloaded, total
        ));
    }

    std::fs::rename(&partial, dest).map_err(|error| {
        let _ = std::fs::remove_file(&partial);
        error.to_string()
    })?;

    Ok(())
}
