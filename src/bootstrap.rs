use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use zip::ZipArchive;

pub const YT_DLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
// A reliable build of ffmpeg for Windows (static linked)
pub const FFMPEG_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";
pub const BIN_DIR: &str = "bin";

pub async fn ensure_dependencies<F>(on_progress: F) -> Result<()>
where
    F: Fn(String, f32) + Send + Sync + 'static,
{
    let bin_path = Path::new(BIN_DIR);
    if !bin_path.exists() {
        fs::create_dir_all(bin_path).context("Failed to create bin directory")?;
    }

    let yt_dlp_path = bin_path.join("yt-dlp.exe");
    if !yt_dlp_path.exists() {
        on_progress("Downloading yt-dlp...".to_string(), 0.1);
        download_file(YT_DLP_URL, &yt_dlp_path, |p| {
            on_progress("Downloading yt-dlp...".to_string(), 0.1 + (p * 0.4));
        })
        .await
        .context("Failed to download yt-dlp")?;
    }

    let ffmpeg_path = bin_path.join("ffmpeg.exe");
    if !ffmpeg_path.exists() {
        on_progress("Downloading ffmpeg...".to_string(), 0.5);
        // Download zip
        let zip_path = bin_path.join("ffmpeg.zip");
         download_file(FFMPEG_URL, &zip_path, |p| {
            on_progress("Downloading ffmpeg...".to_string(), 0.5 + (p * 0.4));
        })
        .await
        .context("Failed to download ffmpeg zip")?;
        
        on_progress("Extracting ffmpeg...".to_string(), 0.9);
        extract_ffmpeg(&zip_path, bin_path).context("Failed to extract ffmpeg")?;
        
        // Clean up zip
        let _ = fs::remove_file(zip_path);
    }

    on_progress("Dependencies ready!".to_string(), 1.0);
    Ok(())
}

async fn download_file<F>(url: &str, dest: &Path, on_progress: F) -> Result<()>
where
    F: Fn(f32),
{
    let client = Client::new();
    let res = client.get(url).send().await?;
    let total_size = res.content_length().unwrap_or(0);

    let mut file = File::create(dest)?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
             let progress = downloaded as f32 / total_size as f32;
             on_progress(progress);
        }
    }
    Ok(())
}

fn extract_ffmpeg(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        // We only care about ffmpeg.exe inside the bin folder of the zip
        if name.ends_with("ffmpeg.exe") {
             // Extract directly to dest_dir/ffmpeg.exe
             let mut out_file = File::create(dest_dir.join("ffmpeg.exe"))?;
             io::copy(&mut file, &mut out_file)?;
             return Ok(());
        }
    }
    Ok(())
}
