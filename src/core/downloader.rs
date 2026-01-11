//! Downloader module - yt-dlp integration

use crate::error::{Result, YtChillError};
use crate::types::DownloadOptions;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::process::Command;

/// Download audio/video using yt-dlp
pub async fn download(url: &str, options: &DownloadOptions) -> Result<()> {
    if !is_command_available("yt-dlp").await {
        return Err(YtChillError::MissingDependency("yt-dlp".into()));
    }

    let mut args = Vec::new();

    // Audio-only by default (download as MP3), unless --video flag is passed
    if !options.video {
        args.extend(["-x", "--audio-format", "mp3"]);
    } else {
        args.extend(["--remux-video", "mp4"]);
    }

    if let Some(ref format) = options.format {
        args.extend(["--format", format]);
    }

    // Output template
    let output_template = format!("{}/%(title)s [%(id)s].%(ext)s", options.output_dir);
    args.extend(["-o", &output_template]);
    args.push(url);

    // Show progress spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message("Downloading...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let status = Command::new("yt-dlp")
        .args(&args)
        .status()
        .await
        .map_err(|e| YtChillError::Spawn(format!("Failed to start yt-dlp: {}", e)))?;

    spinner.finish_and_clear();

    if !status.success() {
        return Err(YtChillError::Spawn(format!(
            "yt-dlp exited with code: {:?}",
            status.code()
        )));
    }

    println!("✓ Download complete!");
    Ok(())
}

/// Check if a command is available in PATH
async fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}
