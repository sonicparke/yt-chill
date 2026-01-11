//! Player module - mpv and syncplay integration

use crate::error::{Result, YtChillError};
use crate::types::PlayOptions;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

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

    // Show snarky buffering message
    print!("⏳ Convincing YouTube to share... 🙄");
    use std::io::Write;
    std::io::stdout().flush().ok();

    // Spawn a background task to show "now playing" after typical buffer time
    let playing_msg_handle = tokio::spawn(async {
        sleep(Duration::from_secs(6)).await;
        // Clear the line and show playing message
        print!("\r\x1b[K");  // Clear current line
        println!("🎵 Vibing... Sit back and chill. (space=pause, q=quit)");
        std::io::stdout().flush().ok();
    });

    // Spawn mpv with inherited stdio so keyboard controls work
    let status = Command::new("mpv")
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::null())  // Suppress mpv's stderr noise
        .status()
        .await
        .map_err(|e| YtChillError::Spawn(format!("Failed to start mpv: {}", e)))?;

    // Cancel the message task if mpv exits quickly (e.g., error or early quit)
    playing_msg_handle.abort();

    if !status.success() {
        // Don't treat user quit (q key) as an error
        if status.code() != Some(4) {
            return Err(YtChillError::Spawn(format!(
                "mpv exited with code: {:?}",
                status.code()
            )));
        }
    }

    // Clear line and show goodbye
    print!("\r\x1b[K");
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

/// Check if a command is available in PATH
async fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}
