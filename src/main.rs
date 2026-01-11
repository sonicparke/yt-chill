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

use crate::core::{downloader, player, youtube};
use crate::storage::{config, history::History};
use crate::types::{AppState, DownloadOptions, MenuItem, PlayOptions, Video};
use crate::ui::selector::{create_selector, detect_selector};
use crate::utils::paths::{ensure_app_dirs, get_history_path};

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

    /// Copy or display the video link
    #[arg(long)]
    copy_url: bool,

    /// Edit the configuration file
    #[arg(short, long)]
    edit: bool,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Ensure app directories exist
    ensure_app_dirs().await?;

    // Handle --edit flag
    if cli.edit {
        let cfg = config::load_config().await?;
        config::edit_config(&cfg.editor).await?;
        return Ok(());
    }

    // Load config
    let cfg = config::load_config().await?;

    // Load history
    let mut history = History::new(&get_history_path(), cfg.max_history_entries);
    history.load().await?;

    // Create selector
    let selector = create_selector(detect_selector());

    // State machine
    let mut state = determine_initial_state(&cli);
    let mut selected_video: Option<Video> = None;
    let query = cli.query.join(" ");

    while state != AppState::Exit {
        match state {
            AppState::Init => {
                // Show main menu
                let menu_items = vec![
                    MenuItem { label: "🔍 Search YouTube".into(), value: AppState::Search },
                    MenuItem { label: "📜 View your history".into(), value: AppState::History },
                    MenuItem { label: "➕ Add subscription".into(), value: AppState::Subscribe },
                    MenuItem { label: "📺 View your feed".into(), value: AppState::Feed },
                ];

                state = selector.select(&menu_items, "Select Action").unwrap_or(AppState::Exit);
            }

            AppState::Search => {
                let search_query = if query.is_empty() {
                    // Prompt for query using dialoguer
                    let input: String = dialoguer::Input::new()
                        .with_prompt("Search YouTube")
                        .interact_text()?;
                    input
                } else {
                    query.clone()
                };

                if search_query.is_empty() {
                    state = AppState::Exit;
                    continue;
                }

                println!("{}", "Searching...".dimmed());
                match youtube::search_videos(&search_query, cli.limit).await {
                    Ok(videos) => {
                        let menu_items: Vec<MenuItem<Video>> = videos
                            .into_iter()
                            .map(|v| MenuItem {
                                label: format_video_label(&v),
                                value: v,
                            })
                            .collect();

                        selected_video = selector.select(&menu_items, "Select Video");
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
                let entries = history.get_all();

                if entries.is_empty() {
                    println!("{}", "No history yet.".yellow());
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

                selected_video = selector.select(&menu_items, "Select from History");
                state = if selected_video.is_some() {
                    AppState::Play
                } else {
                    AppState::Exit
                };
            }

            AppState::Feed => {
                println!("{}", "Feed feature coming soon!".yellow());
                state = AppState::Exit;
            }

            AppState::Subscribe => {
                println!("{}", "Subscribe feature coming soon!".yellow());
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

                // Handle copy URL option
                if cli.copy_url {
                    println!("{} {}", "Video URL:".green(), url);
                    state = AppState::Exit;
                    continue;
                }

                // Determine action based on flags (no menu)
                let action = if cli.download {
                    "download"
                } else if cli.syncplay {
                    "syncplay"
                } else {
                    "stream"  // Default: just play
                };

                println!("{} {}", "Playing:".dimmed(), video.title);

                match action {
                    "stream" => {
                        let opts = PlayOptions {
                            video: cli.video,
                            format: None,
                        };
                        if let Err(e) = player::play(&url, &opts).await {
                            eprintln!("{} {}", "Error:".red(), e);
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
                            eprintln!("{} {}", "Error:".red(), e);
                        }
                    }
                    "syncplay" => {
                        if let Err(e) = player::play_with_syncplay(&url).await {
                            eprintln!("{} {}", "Error:".red(), e);
                        }
                    }
                    _ => {}
                }

                state = AppState::Exit;
            }

            AppState::Exit => break,
        }
    }

    Ok(())
}
