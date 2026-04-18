//! Player module - mpv and syncplay integration

use crate::error::{Result, YtChillError};
use crate::types::PlayOptions;
use crate::utils::process::is_command_available;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Build YouTube URL from video ID
pub fn build_video_url(video_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={}", video_id)
}

/// How interactive mpv playback ended from yt-chill's perspective.
///
/// Keybindings are defined in a minimal [`input.conf`](https://mpv.io/manual/master/#input-conf)
/// (see [`MPV_INPUT_YT_CHILL`]): **b** stops playback and returns to the menu; **q** stops
/// playback and exits yt-chill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpvPlayExit {
    /// Video ended, or user pressed **b** — return to the main menu.
    BackToMenu,
    /// User pressed **q** — exit yt-chill.
    QuitApp,
}

/// Minimal mpv keymap when [`--no-input-default-bindings`](https://mpv.io/manual/master/#options-no-input-default-bindings)
/// is set. Exit codes: **10** = back to menu, **20** = quit app (see `quit` command).
const MPV_INPUT_YT_CHILL: &str = r#"# yt-chill (used with --no-input-default-bindings)
SPACE cycle pause
b quit 10
q quit 20
LEFT seek -5
RIGHT seek 5
UP add volume 2
DOWN add volume -2
"#;

struct MpvInputConfFile(PathBuf);

impl Drop for MpvInputConfFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Play audio/video using mpv with buffering indicator.
pub async fn play(url: &str, options: &PlayOptions) -> Result<MpvPlayExit> {
    if !is_command_available("mpv") {
        return Err(YtChillError::MissingDependency("mpv".into()));
    }

    let config_path =
        std::env::temp_dir().join(format!("yt-chill-mpv-input-{}.conf", std::process::id()));
    std::fs::write(&config_path, MPV_INPUT_YT_CHILL)
        .map_err(|e| YtChillError::Spawn(format!("Failed to write mpv input conf: {}", e)))?;
    let _input_cleanup = MpvInputConfFile(config_path.clone());

    let mut args: Vec<String> = vec![
        "--really-quiet".into(),
        "--no-input-default-bindings".into(),
        "--input-conf".into(),
        config_path.to_string_lossy().into_owned(),
    ];

    if !options.video {
        args.push("--no-video".into());
    }

    if let Some(ref format) = options.format {
        args.push("--ytdl-format".into());
        args.push(format.clone());
    }

    args.push(url.to_string());

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Connecting…");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let vibing_spinner = spinner.clone();
    let vibing_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(6)).await;
        vibing_spinner.finish_and_clear();
        println!("🎵 Playing — space pauses, b back to menu, q quits yt-chill.");
    });

    let status = Command::new("mpv")
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(|e| YtChillError::Spawn(format!("Failed to start mpv: {}", e)))?;

    vibing_handle.abort();
    spinner.finish_and_clear();

    let code = status.code();

    let exit = match code {
        Some(0) | Some(10) => MpvPlayExit::BackToMenu,
        Some(20) => MpvPlayExit::QuitApp,
        None => {
            return Err(YtChillError::Spawn(
                "mpv terminated without an exit code (signal?)".into(),
            ));
        }
        Some(other) => {
            return Err(YtChillError::Spawn(format!(
                "mpv exited with unexpected code: {}",
                other
            )));
        }
    };

    if matches!(exit, MpvPlayExit::BackToMenu) {
        println!("👋 Playback finished. Thanks for chilling.");
    }

    Ok(exit)
}

/// Play with syncplay
pub async fn play_with_syncplay(url: &str) -> Result<()> {
    if !is_command_available("syncplay") {
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
