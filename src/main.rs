//! yt-chill - YouTube audio in your terminal
//!
//! A distraction-free CLI for searching, streaming, and downloading audio from YouTube.

mod core;
mod error;
mod storage;
mod types;
mod ui;
mod utils;

use clap::Parser;
use colored::Colorize;

use crate::core::{
    downloader,
    player::{self, MpvPlayExit},
    youtube,
};
use crate::storage::{config, history::History};
use crate::types::{AppState, Config, DownloadOptions, MenuItem, PlayOptions, Video};
use crate::ui::selector::{create_selector, detect_selector};
use crate::utils::flow::next_state_after_play;
use crate::utils::paths::{ensure_app_dirs, get_history_path};
use crate::utils::playback::use_syncplay;
use crate::utils::process::is_command_available;

/// YouTube audio in your terminal. Clean and distraction-free.
#[derive(Parser, Debug)]
#[command(name = "yt-chill")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Search query
    #[arg(trailing_var_arg = true)]
    query: Vec<String>,

    /// Include video (audio-only by default)
    #[arg(long)]
    video: bool,

    /// Download instead of streaming
    #[arg(short, long)]
    download: bool,

    /// Show and replay from viewing history
    #[arg(long)]
    history: bool,

    /// View videos from your feed
    #[arg(short = 'F', long)]
    feed: bool,

    /// Add a channel to subscriptions
    #[arg(short, long)]
    subscribe: bool,

    /// Watch with friends via syncplay
    #[arg(long)]
    syncplay: bool,

    /// Limit search results
    #[arg(short, long, default_value = "15")]
    limit: usize,

    /// Edit the configuration file
    #[arg(short, long)]
    edit: bool,

    /// List current subscriptions and exit
    #[arg(long)]
    list_subs: bool,

    /// Interactively remove a subscription and exit
    #[arg(long)]
    unsubscribe: bool,

    /// Clear the search/feed cache and exit
    #[arg(long)]
    clear_cache: bool,

    /// Clear watch history and exit
    #[arg(long)]
    clear_history: bool,

    /// Verbose logging (YouTube parse diagnostics, etc.)
    #[arg(short, long)]
    verbose: bool,
}

/// Prompt for a non-empty string, re-prompting on empty input.
///
/// Ctrl-C propagates as `Err` (dialoguer's existing behaviour), which the
/// caller then bubbles up with `?` so the app exits cleanly.
fn prompt_nonempty(prompt: &str) -> anyhow::Result<String> {
    loop {
        let input: String = dialoguer::Input::new()
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()?;
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
        println!(
            "{}",
            "That was empty — try again, or press Ctrl-C to cancel.".yellow()
        );
    }
}

/// Format video for display in selector
fn format_video_label(video: &Video) -> String {
    format!(
        "{} {} - {}",
        video.title,
        format!("[{}]", video.duration).dimmed(),
        video.author.cyan()
    )
}

/// Determine initial state from CLI options
fn determine_initial_state(cli: &Cli) -> AppState {
    if cli.history {
        return AppState::History;
    }
    if cli.feed {
        return AppState::Feed;
    }
    if cli.subscribe {
        return AppState::Subscribe;
    }
    if !cli.query.is_empty() {
        return AppState::Search;
    }
    AppState::Init
}

/// Platform-specific install hint for missing external dependencies.
fn install_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "brew install mpv yt-dlp"
    } else if cfg!(target_os = "linux") {
        "apt install mpv yt-dlp  (or your distro's equivalent)"
    } else {
        "install mpv and yt-dlp from your package manager"
    }
}

/// Fail fast if required external binaries are missing.
fn check_external_deps(cli: &Cli, cfg: &Config) {
    let mut missing: Vec<&str> = Vec::new();

    if !is_command_available("mpv") {
        missing.push("mpv");
    }
    if cli.download && !is_command_available("yt-dlp") {
        missing.push("yt-dlp");
    }
    if use_syncplay(cli.syncplay, cfg.player) && !is_command_available("syncplay") {
        missing.push("syncplay");
    }

    if !missing.is_empty() {
        eprintln!(
            "{} Missing required dependencies: {}",
            "Error:".red(),
            missing.join(", ").yellow()
        );
        eprintln!("  Install with: {}", install_hint().cyan());
        std::process::exit(1);
    }
}

fn init_tracing(verbose: bool) {
    use tracing_subscriber::EnvFilter;

    let filter = if verbose {
        EnvFilter::new("yt_chill=debug")
    } else {
        EnvFilter::new("off")
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    // Ensure app directories exist
    ensure_app_dirs().await?;

    // Handle --edit flag
    if cli.edit {
        let cfg = config::load_config().await?;
        config::edit_config(&cfg.editor).await?;
        return Ok(());
    }

    // Handle subscription management / maintenance flags. These all
    // short-circuit the state machine and exit immediately.
    if cli.list_subs {
        use crate::storage::subscriptions::load_subscriptions;
        let subs = load_subscriptions().await?;
        if subs.is_empty() {
            println!(
                "No subscriptions yet. Use --subscribe or choose “Subscribe to a channel” in the menu."
            );
        } else {
            for sub in &subs {
                println!("{}  {}", sub.handle.cyan(), sub.name.dimmed());
            }
        }
        return Ok(());
    }

    if cli.unsubscribe {
        use crate::storage::subscriptions::{load_subscriptions, remove_subscription};
        let subs = load_subscriptions().await?;
        if subs.is_empty() {
            println!("Nothing to remove — you have no subscriptions yet.");
            return Ok(());
        }

        let selector = create_selector(detect_selector());
        let menu_items: Vec<MenuItem<crate::types::Subscription>> = subs
            .iter()
            .map(|s| MenuItem {
                label: format!("{}  {}", s.handle.cyan(), s.name.dimmed()),
                value: s.clone(),
            })
            .collect();

        if let Some(chosen) = selector.select(&menu_items, "Pick a subscription to remove") {
            remove_subscription(&chosen.handle).await?;
            println!("{} Removed {}", "✓".green(), chosen.name);
        } else {
            println!("{}", "Removal cancelled.".dimmed());
        }
        return Ok(());
    }

    if cli.clear_cache {
        crate::storage::cache::clear_cache().await?;
        println!("{}", "Search and feed cache cleared.".green());
        return Ok(());
    }

    if cli.clear_history {
        let cfg = config::load_config().await?;
        let mut history = History::new(&get_history_path(), cfg.max_history_entries);
        history.load().await?;
        history.clear().await?;
        println!("{}", "Watch history cleared.".green());
        return Ok(());
    }

    // Load config (needed for dependency checks and the main loop).
    let cfg = config::load_config().await?;

    // Fail fast if required external tools are missing.
    check_external_deps(&cli, &cfg);

    // Load history
    let mut history = History::new(&get_history_path(), cfg.max_history_entries);
    history.load().await?;

    // Create selector
    let selector = create_selector(detect_selector());

    // State machine
    let mut state = determine_initial_state(&cli);
    let mut selected_video: Option<Video> = None;
    // Full search result list and cursor when playback started from Search (for auto-advance).
    let mut search_playlist: Option<Vec<Video>> = None;
    let mut search_playlist_index: usize = 0;
    let query = cli.query.join(" ");

    while state != AppState::Exit {
        match state {
            AppState::Init => {
                // Show main menu
                let menu_items = vec![
                    MenuItem {
                        label: "Search YouTube".into(),
                        value: AppState::Search,
                    },
                    MenuItem {
                        label: "Watch history".into(),
                        value: AppState::History,
                    },
                    MenuItem {
                        label: "Subscribe to a channel".into(),
                        value: AppState::Subscribe,
                    },
                    MenuItem {
                        label: "Subscription feed".into(),
                        value: AppState::Feed,
                    },
                ];

                state = selector
                    .select(&menu_items, "What would you like to do?")
                    .unwrap_or(AppState::Exit);
            }

            AppState::Search => {
                search_playlist = None;
                let search_query = if query.is_empty() {
                    prompt_nonempty("YouTube search")?
                } else {
                    query.clone()
                };

                println!("{}", "Searching…".dimmed());
                match youtube::search_videos(&search_query, cli.limit).await {
                    Ok(videos) => {
                        let videos_for_queue = videos.clone();
                        let menu_items: Vec<MenuItem<Video>> = videos
                            .into_iter()
                            .map(|v| MenuItem {
                                label: format_video_label(&v),
                                value: v,
                            })
                            .collect();

                        selected_video = selector.select(&menu_items, "Pick a video");
                        if let Some(v) = &selected_video {
                            if let Some(i) = videos_for_queue.iter().position(|x| x.id == v.id) {
                                search_playlist = Some(videos_for_queue);
                                search_playlist_index = i;
                            } else {
                                search_playlist = None;
                            }
                        }
                        state = if selected_video.is_some() {
                            AppState::Play
                        } else {
                            AppState::Exit
                        };
                    }
                    Err(e) => {
                        eprintln!("{} {}", "Error:".red(), e);
                        state = AppState::Exit;
                    }
                }
            }

            AppState::History => {
                search_playlist = None;
                let entries = history.get_all();

                if entries.is_empty() {
                    println!(
                        "{}",
                        "No watch history yet — play something from Search first.".yellow()
                    );
                    state = AppState::Exit;
                    continue;
                }

                let menu_items: Vec<MenuItem<Video>> = entries
                    .iter()
                    .map(|e| MenuItem {
                        label: format_video_label(&e.video),
                        value: e.video.clone(),
                    })
                    .collect();

                selected_video = selector.select(&menu_items, "Pick a history entry");
                state = if selected_video.is_some() {
                    AppState::Play
                } else {
                    AppState::Exit
                };
            }

            AppState::Feed => {
                use crate::storage::subscriptions::load_subscriptions;

                search_playlist = None;
                // Load subscriptions
                let subs = load_subscriptions().await?;

                if subs.is_empty() {
                    println!(
                        "{}",
                        "No subscriptions yet. Use --subscribe or the menu item above.".yellow()
                    );
                    state = AppState::Exit;
                    continue;
                }

                println!(
                    "{}",
                    format!("Loading videos from {} subscriptions…", subs.len()).dimmed()
                );

                let mut set: tokio::task::JoinSet<(String, crate::error::Result<Vec<Video>>)> =
                    tokio::task::JoinSet::new();
                for sub in &subs {
                    let name = sub.name.clone();
                    let handle = sub.handle.clone();
                    set.spawn(
                        async move { (name, youtube::fetch_channel_videos(&handle, 5).await) },
                    );
                }

                let mut all_videos: Vec<Video> = Vec::new();
                let mut failures: Vec<(String, String)> = Vec::new();
                while let Some(joined) = set.join_next().await {
                    match joined {
                        Ok((_, Ok(videos))) => all_videos.extend(videos),
                        Ok((name, Err(e))) => failures.push((name, e.to_string())),
                        Err(join_err) => failures.push(("<task>".into(), join_err.to_string())),
                    }
                }

                for (name, err) in &failures {
                    eprintln!("{} {}: {}", "warn:".yellow(), name, err);
                }

                if all_videos.is_empty() {
                    println!(
                        "{}",
                        "No videos loaded — check your connection or try again later.".yellow()
                    );
                    state = AppState::Exit;
                    continue;
                }

                // Sort by... nothing for now, just show them
                let menu_items: Vec<MenuItem<Video>> = all_videos
                    .into_iter()
                    .map(|v| MenuItem {
                        label: format_video_label(&v),
                        value: v,
                    })
                    .collect();

                selected_video = selector.select(&menu_items, "Pick a feed video");
                state = if selected_video.is_some() {
                    AppState::Play
                } else {
                    AppState::Exit
                };
            }

            AppState::Subscribe => {
                use crate::storage::subscriptions::add_subscription;
                use crate::types::Subscription;

                // Prompt for channel search (re-prompts on empty input;
                // Ctrl-C exits cleanly via `?`).
                let search_query = prompt_nonempty("Channel name or topic")?;

                println!("{}", "Searching for channels…".dimmed());
                match youtube::search_channels(&search_query, 10).await {
                    Ok(channels) => {
                        let menu_items: Vec<MenuItem<youtube::ChannelInfo>> = channels
                            .into_iter()
                            .map(|c| MenuItem {
                                label: format!("{} ({})", c.name, c.handle.cyan()),
                                value: c,
                            })
                            .collect();

                        if let Some(channel) = selector.select(&menu_items, "Pick a channel") {
                            let sub = Subscription {
                                name: channel.name.clone(),
                                handle: channel.handle.clone(),
                            };

                            match add_subscription(&sub).await {
                                Ok(_) => {
                                    println!("{} Subscribed to {}", "✓".green(), channel.name);
                                }
                                Err(e) => {
                                    eprintln!("{} Failed to subscribe: {}", "Error:".red(), e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{} {}", "Error:".red(), e);
                    }
                }
                state = AppState::Exit;
            }

            AppState::Play => {
                let Some(ref video) = selected_video else {
                    state = AppState::Exit;
                    continue;
                };

                let url = player::build_video_url(&video.id);

                // Add to history
                history.add(video).await?;

                // Determine action based on flags (no menu)
                let action = if cli.download {
                    "download"
                } else if use_syncplay(cli.syncplay, cfg.player) {
                    "syncplay"
                } else {
                    "stream" // Default: just play
                };

                println!("{} {}", "Now playing:".dimmed(), video.title);

                let mut play_ok = true;
                match action {
                    "stream" => {
                        let opts = PlayOptions {
                            video: cli.video,
                            format: None,
                            verbose: cli.verbose,
                        };
                        match player::play(&url, &opts).await {
                            Ok(MpvPlayExit::StreamEnded) => {
                                let advanced = if let Some(list) = &search_playlist {
                                    if search_playlist_index + 1 < list.len() {
                                        search_playlist_index += 1;
                                        selected_video = Some(list[search_playlist_index].clone());
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };
                                if advanced {
                                    let n = search_playlist_index + 1;
                                    let total =
                                        search_playlist.as_ref().map(|l| l.len()).unwrap_or(0);
                                    let title = selected_video
                                        .as_ref()
                                        .map(|v| v.title.as_str())
                                        .unwrap_or("");
                                    println!(
                                        "{}",
                                        format!("Up next ({n}/{total}): {title}").dimmed()
                                    );
                                    state = AppState::Play;
                                    continue;
                                }
                                println!("👋 Playback finished. Thanks for chilling.");
                                search_playlist = None;
                                state = next_state_after_play(true);
                                continue;
                            }
                            Ok(MpvPlayExit::BackToMenu) => {
                                println!("👋 Playback finished. Thanks for chilling.");
                                search_playlist = None;
                                state = next_state_after_play(true);
                                continue;
                            }
                            Ok(MpvPlayExit::QuitApp) => {
                                search_playlist = None;
                                state = next_state_after_play(false);
                                continue;
                            }
                            Err(e) => {
                                search_playlist = None;
                                eprintln!("{} {}", "Error:".red(), e);
                                state = next_state_after_play(false);
                                continue;
                            }
                        }
                    }
                    "download" => {
                        let download_dir = if cfg.download_dir.is_empty() {
                            dirs::download_dir()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| ".".into())
                        } else {
                            cfg.download_dir.clone()
                        };

                        let opts = DownloadOptions {
                            video: cli.video,
                            format: None,
                            output_dir: download_dir,
                        };
                        if let Err(e) = downloader::download(&url, &opts).await {
                            play_ok = false;
                            eprintln!("{} {}", "Error:".red(), e);
                        }
                    }
                    "syncplay" => {
                        if let Err(e) = player::play_with_syncplay(&url).await {
                            play_ok = false;
                            eprintln!("{} {}", "Error:".red(), e);
                        }
                    }
                    _ => {}
                }

                state = next_state_after_play(play_ok);
            }

            AppState::Exit => break,
        }
    }

    Ok(())
}
