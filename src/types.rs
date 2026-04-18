//! Type definitions for yt-chill
//!
//! Source of truth for all data structures.

use serde::{Deserialize, Serialize};

// ============================================
// Video Types
// ============================================

/// A video result from YouTube search or feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub author: String,
    /// Duration formatted as "3:45" or "1:23:45"
    pub duration: String,
    /// Raw from YouTube, e.g., "1.2M views"
    pub views: String,
    /// Raw from YouTube, e.g., "2 days ago"
    pub published: String,
    /// URL to thumbnail image
    pub thumbnail: String,
}

/// A video in watch history (extends Video with timestamp)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    #[serde(flatten)]
    pub video: Video,
    /// Unix timestamp when watched
    pub timestamp: i64,
}

/// A channel result from channel search
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    /// @channelhandle
    pub handle: String,
    pub thumbnail: String,
    /// e.g., "1.2M subscribers"
    pub subscribers: String,
    /// e.g., "500 videos"
    pub video_count: String,
}

/// A subscription entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Display name
    pub name: String,
    /// @handle or /c/channelname or /channel/ID
    pub handle: String,
}

// ============================================
// Config Types
// ============================================

/// Video player type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PlayerType {
    #[default]
    Mpv,
    Syncplay,
}

/// Menu selector type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SelectorType {
    #[default]
    Fzf,
    Dialoguer,
}

/// User configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Search result limit (default: 15)
    pub limit: usize,
    /// Default to video mode (false = audio-only, which is the default)
    pub video_mode: bool,
    /// Download directory path
    pub download_dir: String,
    /// Max history entries (default: 100)
    pub max_history_entries: usize,
    /// Editor command (default: `$EDITOR`, falling back to `vi`)
    pub editor: String,
    /// Video player
    pub player: PlayerType,
    /// Menu selector
    pub selector: SelectorType,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            limit: 15,
            video_mode: false,           // Audio-only by default
            download_dir: String::new(), // Set at runtime to ~/Downloads
            max_history_entries: 100,
            editor: std::env::var("EDITOR").unwrap_or_else(|_| "vi".into()),
            player: PlayerType::default(),
            selector: SelectorType::default(),
        }
    }
}

// ============================================
// Playback Types
// ============================================

/// Options for playback
#[derive(Debug, Clone, Default)]
pub struct PlayOptions {
    /// Include video (false = audio-only)
    pub video: bool,
    /// yt-dlp format string
    pub format: Option<String>,
}

/// Options for video download
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Include video (false = audio-only)
    pub video: bool,
    /// yt-dlp format string
    pub format: Option<String>,
    /// Output directory
    pub output_dir: String,
}

/// Available video format/quality
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct VideoFormat {
    /// Format code for yt-dlp
    pub code: String,
    /// "1080p", "720p", etc.
    pub resolution: String,
    /// Framerate if available
    pub fps: Option<u32>,
    /// Estimated size
    pub filesize: Option<String>,
}

// ============================================
// Selector Types
// ============================================

/// Item displayed in selector menu
#[derive(Debug, Clone)]
pub struct MenuItem<T> {
    /// Display text
    pub label: String,
    /// Underlying value
    pub value: T,
}

// ============================================
// State Machine Types
// ============================================

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Initial menu selection
    Init,
    /// Search and select video
    Search,
    /// Browse history
    History,
    /// Browse subscription feed
    Feed,
    /// Add subscription
    Subscribe,
    /// Play/download selected video
    Play,
    /// Exit application
    Exit,
}

// ============================================
// Cache Types
// ============================================

/// Cached data with TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub timestamp: i64,
    /// Time-to-live in seconds
    pub ttl: u64,
}
