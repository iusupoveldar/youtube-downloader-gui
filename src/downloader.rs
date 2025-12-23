use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::path::Path;
use std::io::{BufRead, BufReader};
use std::thread;

use std::sync::Arc;

pub fn start_download<F>(url: String, format: String, quality: String, on_output: F) -> Result<()> 
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
    // Simplified for now
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
    
    // Output template
    args.push("-o".to_string());
    args.push("downloads/%(title)s.%(ext)s".to_string());

    args.push(url);

    // Spawn process
    let mut child = Command::new(yt_dlp_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // Capture stderr too as yt-dlp often prints progress there
        .spawn()
        .context("Failed to start download process")?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let on_output = Arc::new(on_output);
    let on_output_stdout = on_output.clone();
    let on_output_stderr = on_output.clone();

    // Stream output (simple blocking thread for now, can be improved)
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                on_output_stdout(l);
            }
        }
    });

    // Also read stderr for progress
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
             if let Ok(l) = line {
                // simple progress filter, or just pipe everything
                on_output_stderr(l);
            }
        }
    });

    // We don't wait for child here in the main thread to avoid blocking UI
    // In a real app we'd want to manage the child process better (wait in thread and report finish)
    
    Ok(())
}
