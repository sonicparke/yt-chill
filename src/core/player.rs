//! Player module - mpv and syncplay integration

use crate::error::{Result, YtChillError};
use crate::types::PlayOptions;
use crate::utils::process::is_command_available;
use indicatif::{ProgressBar, ProgressStyle};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Build YouTube URL from video ID
pub fn build_video_url(video_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={}", video_id)
}

/// Play audio/video using mpv with buffering indicator
pub async fn play(url: &str, options: &PlayOptions) -> Result<()> {
    // Check if mpv is available
    if !is_command_available("mpv").await {
        return Err(YtChillError::MissingDependency("mpv".into()));
    }

    let mut args = vec!["--really-quiet"];

    // Audio-only by default, unless --video flag is passed
    if !options.video {
        args.push("--no-video");
    }

    if let Some(ref format) = options.format {
        args.push("--ytdl-format");
        args.push(format);
    }

    args.push(url);

    // Buffering spinner: runs until either the 6s "now vibing" transition
    // fires or mpv exits early.
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Buffering...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    // After ~6s (typical time-to-first-byte for mpv+ytdl), swap the spinner
    // out for the cheerful "Vibing..." line. If mpv exits first, we abort
    // this task below so the message never prints.
    let vibing_spinner = spinner.clone();
    let vibing_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(6)).await;
        vibing_spinner.finish_and_clear();
        println!("🎵 Vibing... Sit back and chill. (space=pause, q=quit)");
    });

    // Spawn mpv with inherited stdio so keyboard controls work
    let status = Command::new("mpv")
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::null()) // Suppress mpv's stderr noise
        .status()
        .await
        .map_err(|e| YtChillError::Spawn(format!("Failed to start mpv: {}", e)))?;

    // If mpv exited before the "Vibing..." task fired, cancel it and clear
    // the spinner ourselves. If it already fired, finish_and_clear is a
    // no-op since the bar is already finished.
    vibing_handle.abort();
    spinner.finish_and_clear();

    if !status.success() {
        return Err(YtChillError::Spawn(format!(
            "mpv exited with code: {:?}",
            status.code()
        )));
    }

    println!("👋 Thanks for chilling.");

    Ok(())
}

/// Play with syncplay
pub async fn play_with_syncplay(url: &str) -> Result<()> {
    if !is_command_available("syncplay").await {
        return Err(YtChillError::MissingDependency("syncplay".into()));
    }

    let status = Command::new("syncplay")
        .arg(url)
        .status()
        .await
        .map_err(|e| YtChillError::Spawn(format!("Failed to start syncplay: {}", e)))?;

    if !status.success() {
        return Err(YtChillError::Spawn(format!(
            "syncplay exited with code: {:?}",
            status.code()
        )));
    }

    Ok(())
}
