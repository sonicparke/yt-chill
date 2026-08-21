//! Player module - mpv and syncplay integration

use crate::error::{Result, YtChillError};
use crate::types::PlayOptions;
use crate::utils::process::{is_command_available, resolve_on_path};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Successful mpv playback plus the last volume observed by yt-chill's helper script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpvPlayOutcome {
    pub exit: MpvPlayExit,
    pub final_volume: Option<u8>,
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

// mpv documents `observe_property` as delivering an initial value and subsequent
// changes. Writing on each observation also preserves the latest value if mpv
// cannot deliver an orderly shutdown event.
// Source: https://mpv.io/manual/master/#lua-scripting-mp-observe-property
const MPV_VOLUME_OBSERVER: &str = r#"local state_path = os.getenv("YT_CHILL_VOLUME_STATE")

local function save_volume(_, value)
    if not state_path or state_path == "" or value == nil then
        return
    end

    local rounded = math.floor(value + 0.5)
    if rounded < 0 or rounded > 100 then
        return
    end

    local file = io.open(state_path, "w")
    if file then
        file:write(tostring(rounded))
        file:close()
    end
end

mp.observe_property("volume", "number", save_volume)
mp.register_event("shutdown", function()
    save_volume("volume", mp.get_property_number("volume"))
end)
"#;

const HOMEBREW_MEDIA_REPAIR_HINT: &str =
    "Homebrew media libraries look out of sync. Try: brew reinstall x265 ffmpeg mpv";

static MPV_RUNTIME_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_private_dir(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700).create(path)
    }

    #[cfg(not(unix))]
    {
        std::fs::create_dir(path)
    }
}

struct MpvRuntimeFiles {
    dir: PathBuf,
    input_conf: PathBuf,
    volume_observer: PathBuf,
    volume_state: PathBuf,
}

impl MpvRuntimeFiles {
    fn create() -> Result<Self> {
        let sequence = MPV_RUNTIME_COUNTER.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir();
        let mut created_dir = None;

        for attempt in 0..16_u8 {
            let dir = base.join(format!(
                "yt-chill-mpv-{}-{sequence}-{attempt}",
                std::process::id()
            ));
            match create_private_dir(&dir) {
                Ok(()) => {
                    created_dir = Some(dir);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(YtChillError::Spawn(format!(
                        "Failed to create mpv runtime directory: {error}"
                    )));
                }
            }
        }

        let dir = created_dir.ok_or_else(|| {
            YtChillError::Spawn("Failed to create a unique mpv runtime directory".into())
        })?;
        let files = Self {
            input_conf: dir.join("input.conf"),
            volume_observer: dir.join("volume-observer.lua"),
            volume_state: dir.join("volume.state"),
            dir,
        };

        std::fs::write(&files.input_conf, MPV_INPUT_YT_CHILL)
            .and_then(|_| std::fs::write(&files.volume_observer, MPV_VOLUME_OBSERVER))
            .map_err(|error| {
                YtChillError::Spawn(format!("Failed to write mpv runtime files: {error}"))
            })?;

        Ok(files)
    }

    fn observed_volume(&self) -> Option<u8> {
        std::fs::read_to_string(&self.volume_state)
            .ok()
            .and_then(|raw| parse_observed_volume(&raw))
    }
}

impl Drop for MpvRuntimeFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.input_conf);
        let _ = std::fs::remove_file(&self.volume_observer);
        let _ = std::fs::remove_file(&self.volume_state);
        let _ = std::fs::remove_dir(&self.dir);
    }
}

fn parse_observed_volume(raw: &str) -> Option<u8> {
    raw.trim()
        .parse::<u8>()
        .ok()
        .filter(|volume| *volume <= 100)
}

fn build_mpv_args(
    url: &str,
    options: &PlayOptions,
    input_conf_path: &Path,
    volume_observer_path: &Path,
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
    args.push(format!(
        "--script={}",
        volume_observer_path.to_string_lossy()
    ));
    // mpv's volume is its internal software mixer, so system output volume
    // remains a downstream multiplier.
    // Source: https://mpv.io/manual/master/#options-volume
    args.push(format!("--volume={}", options.volume));
    args.push("--volume-max=100".into());

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

fn mpv_outcome_from_status(
    code: Option<i32>,
    final_volume: Option<u8>,
    url: &str,
) -> Result<MpvPlayOutcome> {
    let exit = match code {
        Some(0) => MpvPlayExit::StreamEnded,
        Some(10) => MpvPlayExit::BackToMenu,
        Some(20) => MpvPlayExit::QuitApp,
        None => return Err(mpv_termination_error(&[])),
        Some(other) => return Err(mpv_unexpected_exit_error(other, url)),
    };

    Ok(MpvPlayOutcome { exit, final_volume })
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
pub async fn play(url: &str, options: &PlayOptions) -> Result<MpvPlayOutcome> {
    if !is_command_available("mpv") {
        return Err(YtChillError::MissingDependency("mpv".into()));
    }

    check_mpv_starts().await?;

    let runtime_files = MpvRuntimeFiles::create()?;

    let ytdl_path = resolve_on_path("yt-dlp");
    let args = build_mpv_args(
        url,
        options,
        &runtime_files.input_conf,
        &runtime_files.volume_observer,
        ytdl_path.as_deref(),
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Connecting…");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let vibing_spinner = spinner.clone();
    let starting_volume = options.volume;
    let vibing_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(6)).await;
        vibing_spinner.finish_and_clear();
        println!(
            "🎵 Playing — yt-chill started at {starting_volume}% (relative to system); ↑/↓ volume, space pause, b menu, q quit."
        );
    });

    let status = Command::new("mpv")
        .args(&args)
        .env("YT_CHILL_VOLUME_STATE", &runtime_files.volume_state)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        // YouTube / ytdl / audio errors go to stderr; discarding them made exit 1 impossible to debug.
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| YtChillError::Spawn(format!("Failed to start mpv: {}", e)))?;

    vibing_handle.abort();
    spinner.finish_and_clear();

    mpv_outcome_from_status(status.code(), runtime_files.observed_volume(), url)
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
            Path::new("/tmp/yt-chill-volume.lua"),
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
            volume: 100,
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
            volume: 100,
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
    fn playback_volume_is_internal_and_capped_at_full_scale() {
        let args = args_for(PlayOptions {
            video: false,
            format: None,
            verbose: false,
            volume: 50,
        });

        assert!(args.iter().any(|arg| arg == "--volume=50"));
        assert!(args.iter().any(|arg| arg == "--volume-max=100"));
        assert_eq!(
            args.iter()
                .filter(|arg| arg.starts_with("--volume="))
                .count(),
            1
        );
        assert_inline_option(&args, "--volume");
        assert_inline_option(&args, "--volume-max");
    }

    #[test]
    fn default_playback_volume_preserves_existing_full_scale_behavior() {
        assert_eq!(PlayOptions::default().volume, 100);
    }

    #[test]
    fn playback_loads_volume_observer_with_a_state_file() {
        let args = args_for(PlayOptions {
            volume: 50,
            ..PlayOptions::default()
        });

        assert!(
            args.iter()
                .any(|arg| arg == "--script=/tmp/yt-chill-volume.lua")
        );
        assert!(
            !args
                .iter()
                .any(|arg| arg.starts_with("--input-ipc-server="))
        );
    }

    #[test]
    fn observed_volume_uses_last_valid_bounded_value() {
        assert_eq!(parse_observed_volume("48\n"), Some(48));
        assert_eq!(parse_observed_volume("0"), Some(0));
        assert_eq!(parse_observed_volume("100"), Some(100));
    }

    #[test]
    fn invalid_observed_volume_is_not_persistable() {
        assert_eq!(parse_observed_volume("101"), None);
        assert_eq!(parse_observed_volume("not-a-volume"), None);
        assert_eq!(parse_observed_volume(""), None);
    }

    #[test]
    fn mpv_runtime_files_are_removed_after_playback_scope() {
        let runtime_dir;
        {
            let files = MpvRuntimeFiles::create().unwrap();
            runtime_dir = files.dir.clone();
            assert!(files.input_conf.exists());
            assert!(files.volume_observer.exists());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    std::fs::metadata(&runtime_dir)
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o700
                );
            }
        }

        assert!(!runtime_dir.exists());
    }

    #[test]
    fn successful_mpv_exits_preserve_observed_volume() {
        for (code, expected_exit) in [
            (0, MpvPlayExit::StreamEnded),
            (10, MpvPlayExit::BackToMenu),
            (20, MpvPlayExit::QuitApp),
        ] {
            let outcome =
                mpv_outcome_from_status(Some(code), Some(48), "https://example.test/video")
                    .unwrap();
            assert_eq!(outcome.exit, expected_exit);
            assert_eq!(outcome.final_volume, Some(48));
        }
    }

    #[test]
    fn audio_only_without_custom_format_requests_default_audio_format() {
        let args = args_for(PlayOptions {
            video: false,
            format: None,
            verbose: false,
            volume: 100,
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
            volume: 100,
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
            volume: 100,
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
            volume: 100,
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
