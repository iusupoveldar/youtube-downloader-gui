use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::path::Path;
use std::io::{BufRead, BufReader};
use std::thread;

use std::sync::Arc;

pub fn start_download<F>(url: String, format: String, quality: String, download_path: String, on_output: F) -> Result<()> 
where
    F: Fn(String) + Send + Sync + 'static,
{
    let bin_path = Path::new("bin");
    let yt_dlp_path = bin_path.join("yt-dlp.exe");
    let _ffmpeg_path = bin_path.join("ffmpeg.exe"); // yt-dlp should automatically find this if in same dir, or we add to path

    if !yt_dlp_path.exists() {
        return Err(anyhow::anyhow!("yt-dlp not found. Please restart app to bootstrap."));
    }

    let mut args = vec![];

    // Format selection logic
    match format.as_str() {
        "Audio (MP3)" => {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push("mp3".to_string());
        }
         _ => {
            args.push("-f".to_string());
             // Quality selection logic
            match quality.as_str() {
                "Best" => args.push("bestvideo+bestaudio/best".to_string()),
                "1080p" => args.push("bestvideo[height<=1080]+bestaudio/best[height<=1080]".to_string()),
                "720p" => args.push("bestvideo[height<=720]+bestaudio/best[height<=720]".to_string()),
                "480p" => args.push("bestvideo[height<=480]+bestaudio/best[height<=480]".to_string()),
                _ => args.push("best".to_string()), // Default
            };
            args.push("--merge-output-format".to_string());
            args.push("mp4".to_string());
        }
    }

    // Set ffmpeg location explicitly to be safe
    args.push("--ffmpeg-location".to_string());
    args.push(bin_path.to_string_lossy().to_string());
    
    // Download Path (Use -P for paths in yt-dlp)
    args.push("-P".to_string());
    args.push(download_path);

    // Output template (filename only, path handled by -P)
    args.push("-o".to_string());
    args.push("%(title)s.%(ext)s".to_string());

    args.push(url);

    // Spawn process
    // Use creation_flags to hide window on Windows if desired, but for now standard spawn
    let mut child = Command::new(yt_dlp_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) 
        .spawn()
        .context("Failed to start download process")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let on_output = Arc::new(on_output);
    let on_output_stdout = on_output.clone();
    let on_output_stderr = on_output.clone();

    // Stream output 
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                on_output_stdout(l);
            }
        }
    });

    // Also read stderr 
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
             if let Ok(l) = line {
                on_output_stderr(l);
            }
        }
    });

    Ok(())
}
