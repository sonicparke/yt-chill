//! Player module - mpv and syncplay integration

use crate::error::{Result, YtChillError};
use crate::types::PlayOptions;
use crate::utils::process::{is_command_available, resolve_on_path};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Default yt-dlp format for audio-only playback.
///
/// Audio mode should request an audio-only YouTube stream instead of relying on
/// `--no-video`, which merely hides video after mpv/yt-dlp have selected media.
pub const AUDIO_ONLY_FORMAT: &str = "bestaudio[abr<=160]/bestaudio/best";

/// Conservative video format candidate for future config-driven video defaults.
///
/// This is intentionally not forced today; video playback only uses a format
/// when the user config provides one.
#[allow(dead_code)]
pub const VIDEO_FORMAT: &str = "bestvideo[height<=720]+bestaudio/best[height<=720]/best";

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
    /// mpv exit **0** — stream or file ended normally (may advance a search playlist).
    StreamEnded,
    /// mpv exit **10** — user pressed **b**, return to the main menu.
    BackToMenu,
    /// mpv exit **20** — user pressed **q**, exit yt-chill.
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

const HOMEBREW_MEDIA_REPAIR_HINT: &str =
    "Homebrew media libraries look out of sync. Try: brew reinstall x265 ffmpeg mpv";

struct MpvInputConfFile(PathBuf);

impl Drop for MpvInputConfFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn build_mpv_args(
    url: &str,
    options: &PlayOptions,
    input_conf_path: &Path,
    ytdl_path: Option<&Path>,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    if !options.verbose {
        // Keep mpv's normal chatter down without suppressing the yt-dlp/ffmpeg
        // errors that explain playback failures.
        args.push("--quiet".into());
    }

    args.push("--no-input-default-bindings".into());
    args.push(format!(
        "--input-conf={}",
        input_conf_path.to_string_lossy()
    ));

    // Pin mpv's ytdl_hook to the yt-dlp on PATH. Without this, mpv may fall back to
    // a stale bundled youtube-dl and fail on videos the newer yt-dlp can extract
    // (e.g. ones where web/web_safari return UNPLAYABLE but android_vr works).
    if let Some(ytdl) = ytdl_path {
        args.push(format!(
            "--script-opts=ytdl_hook-ytdl_path={}",
            ytdl.to_string_lossy()
        ));
    }

    if !options.video {
        args.push("--no-video".into());
    }

    let format = if options.video {
        options.format.as_deref()
    } else {
        Some(options.format.as_deref().unwrap_or(AUDIO_ONLY_FORMAT))
    };

    if let Some(format) = format {
        args.push(format!("--ytdl-format={format}"));
    }

    args.push(url.to_string());
    args
}

fn mpv_termination_error(stderr: &[u8]) -> YtChillError {
    let stderr = String::from_utf8_lossy(stderr);

    if stderr.contains("Library not loaded:") {
        let detail = stderr
            .lines()
            .find_map(|line| {
                line.find("Library not loaded:")
                    .map(|start| line[start..].trim())
            })
            .unwrap_or("Library not loaded");

        return YtChillError::Spawn(format!(
            "mpv could not start because a dynamic library is missing ({detail}). {HOMEBREW_MEDIA_REPAIR_HINT}"
        ));
    }

    YtChillError::Spawn("mpv terminated without an exit code (signal?)".into())
}

fn mpv_unexpected_exit_error(code: i32, url: &str) -> YtChillError {
    YtChillError::Playback(format!(
        "mpv exited with code {code}. See the mpv/yt-dlp error above; re-run with --verbose for more detail or try: mpv {url:?}"
    ))
}

async fn check_mpv_starts() -> Result<()> {
    let output = Command::new("mpv")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| YtChillError::Spawn(format!("Failed to start mpv: {}", e)))?;

    match output.status.code() {
        Some(0) => Ok(()),
        None => Err(mpv_termination_error(&output.stderr)),
        Some(code) => Err(YtChillError::Spawn(format!(
            "mpv --version exited with code: {code}"
        ))),
    }
}

/// Play audio/video using mpv with buffering indicator.
pub async fn play(url: &str, options: &PlayOptions) -> Result<MpvPlayExit> {
    if !is_command_available("mpv") {
        return Err(YtChillError::MissingDependency("mpv".into()));
    }

    check_mpv_starts().await?;

    let config_path =
        std::env::temp_dir().join(format!("yt-chill-mpv-input-{}.conf", std::process::id()));
    std::fs::write(&config_path, MPV_INPUT_YT_CHILL)
        .map_err(|e| YtChillError::Spawn(format!("Failed to write mpv input conf: {}", e)))?;
    let _input_cleanup = MpvInputConfFile(config_path.clone());

    let ytdl_path = resolve_on_path("yt-dlp");
    let args = build_mpv_args(url, options, &config_path, ytdl_path.as_deref());

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
        // YouTube / ytdl / audio errors go to stderr; discarding them made exit 1 impossible to debug.
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| YtChillError::Spawn(format!("Failed to start mpv: {}", e)))?;

    vibing_handle.abort();
    spinner.finish_and_clear();

    let code = status.code();

    let exit = match code {
        Some(0) => MpvPlayExit::StreamEnded,
        Some(10) => MpvPlayExit::BackToMenu,
        Some(20) => MpvPlayExit::QuitApp,
        None => return Err(mpv_termination_error(&[])),
        Some(other) => return Err(mpv_unexpected_exit_error(other, url)),
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn args_for(options: PlayOptions) -> Vec<String> {
        build_mpv_args(
            "https://www.youtube.com/watch?v=test",
            &options,
            Path::new("/tmp/yt-chill-input.conf"),
            Some(Path::new("/usr/local/bin/yt-dlp")),
        )
    }

    fn ytdl_format_values(args: &[String]) -> Vec<&str> {
        args.iter()
            .filter_map(|arg| arg.strip_prefix("--ytdl-format="))
            .collect()
    }

    fn assert_single_ytdl_format(args: &[String], expected: &str) {
        let values = ytdl_format_values(args);
        assert_eq!(values, vec![expected]);
        assert_eq!(
            args.iter()
                .filter(|arg| arg.starts_with("--ytdl-format="))
                .count(),
            1
        );
    }

    fn assert_inline_option(args: &[String], option: &str) {
        assert!(
            !args.iter().any(|arg| arg == option),
            "mpv requires inline values for {option}; use {option}=value, not {option} value"
        );
        assert!(
            args.iter()
                .any(|arg| arg.starts_with(&format!("{option}="))),
            "expected an inline {option}=value argument in {args:?}"
        );
    }

    #[test]
    fn mpv_value_options_use_inline_equals_syntax() {
        let args = args_for(PlayOptions {
            video: false,
            format: Some("251/bestaudio".into()),
            verbose: false,
        });

        assert_inline_option(&args, "--input-conf");
        assert_inline_option(&args, "--script-opts");
        assert_inline_option(&args, "--ytdl-format");
    }

    #[test]
    fn non_verbose_playback_keeps_mpv_errors_visible() {
        let args = args_for(PlayOptions {
            video: false,
            format: None,
            verbose: false,
        });

        assert!(args.iter().any(|arg| arg == "--quiet"));
        assert!(!args.iter().any(|arg| arg == "--really-quiet"));
    }

    #[test]
    fn unexpected_mpv_exit_is_reported_as_playback_failure() {
        let error = mpv_unexpected_exit_error(2, "https://example.test/video");
        let message = error.to_string();

        assert!(message.starts_with("Playback failed:"));
        assert!(!message.contains("Failed to spawn process"));
    }

    #[test]
    fn audio_only_without_custom_format_requests_default_audio_format() {
        let args = args_for(PlayOptions {
            video: false,
            format: None,
            verbose: false,
        });

        assert!(args.iter().any(|arg| arg == "--no-video"));
        assert_single_ytdl_format(&args, AUDIO_ONLY_FORMAT);
    }

    #[test]
    fn audio_only_with_custom_format_uses_custom_format() {
        let args = args_for(PlayOptions {
            video: false,
            format: Some("251/bestaudio".into()),
            verbose: false,
        });

        assert!(args.iter().any(|arg| arg == "--no-video"));
        assert_single_ytdl_format(&args, "251/bestaudio");
        assert!(!ytdl_format_values(&args).contains(&AUDIO_ONLY_FORMAT));
    }

    #[test]
    fn video_without_custom_format_does_not_force_video_or_ytdl_format() {
        let args = args_for(PlayOptions {
            video: true,
            format: None,
            verbose: false,
        });

        assert!(!args.iter().any(|arg| arg == "--no-video"));
        assert!(!args.iter().any(|arg| arg.starts_with("--ytdl-format")));
        assert!(ytdl_format_values(&args).is_empty());
    }

    #[test]
    fn video_with_custom_format_includes_single_custom_ytdl_format() {
        let args = args_for(PlayOptions {
            video: true,
            format: Some(VIDEO_FORMAT.into()),
            verbose: false,
        });

        assert!(!args.iter().any(|arg| arg == "--no-video"));
        assert_single_ytdl_format(&args, VIDEO_FORMAT);
    }

    #[test]
    fn mpv_signal_error_reports_missing_dynamic_library_repair_hint() {
        let error = mpv_termination_error(
            b"dyld[91327]: Library not loaded: /opt/homebrew/opt/x265/lib/libx265.215.dylib\n",
        );

        assert!(error.to_string().contains("Library not loaded:"));
        assert!(error.to_string().contains("brew reinstall x265 ffmpeg mpv"));
    }
}
