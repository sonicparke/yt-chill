# yt-chill: Build Prompt

> This prompt describes how to build yt-chill from scratch in Rust.

## Overview

Build a command-line YouTube audio player in Rust called `yt-chill`. It should be distraction-free, audio-first, and installable globally via `cargo install`.

## Core Behavior

- **Audio-only by default** — streams audio, no video
- **Streaming by default** — no download unless `-d` flag
- **No menus after selection** — pick a video, it plays immediately
- **Keyboard controls work** — space to pause, q to quit, arrows to seek
- **Main menu** — if no query given, show interactive menu (Search, History, Subscribe, Feed)

## CLI Flags

```
yt-chill "search query"      # Search → select → stream audio
yt-chill --video "query"     # Include video
yt-chill -d "query"          # Download as MP3 instead of stream
yt-chill -d --video "query"  # Download as MP4
yt-chill --history           # Browse watch history
yt-chill --syncplay "query"  # Watch with friends via syncplay
yt-chill -e                  # Edit config file
yt-chill --copy-url "query"  # Display video URL
yt-chill -l 20 "query"       # Limit results (default: 15)
yt-chill -F, --feed          # View subscription feed
yt-chill -s, --subscribe     # Add channel to subscriptions
```

## Technical Requirements

### YouTube Integration
- Scrape YouTube HTML directly (no API key required)
- Parse `ytInitialData` JSON from response using regex
- Use browser-like User-Agent header to avoid blocks
- Cache search results with SHA256 key and 1-hour TTL

### Dependencies (External)
- `mpv` for playback
- `yt-dlp` for downloads
- `fzf` (optional) for better selection UI, fallback to built-in prompts

### User Experience
- Show snarky buffering message: `⏳ Convincing YouTube to share... 🙄`
- After 6 seconds show: `🎵 Vibing... Sit back and chill. (space=pause, q=quit)`
- On exit show: `👋 Thanks for chilling.`
- Use colored terminal output (`colored` crate)
- Support fzf for video selection, fall back to dialoguer
- Format video labels: `Title [duration] - Author`

### Storage (XDG Compliant)
- Config: `~/.config/yt-chill/config.json`
- History: `~/.cache/yt-chill/history.json`
- Cache: `~/.cache/yt-chill/<sha256>.json`
- Subscriptions: `~/.config/yt-chill/subscriptions.txt`
- Respect `XDG_CONFIG_HOME` and `XDG_CACHE_HOME` environment variables

### Module Structure
```
src/
├── main.rs          # CLI parsing + state machine
├── lib.rs           # Module exports
├── types.rs         # All data structures
├── error.rs         # Custom error types (thiserror)
├── core/
│   ├── youtube.rs   # YouTube scraping + parsing
│   ├── player.rs    # mpv integration
│   └── downloader.rs # yt-dlp wrapper
├── storage/
│   ├── config.rs    # JSON config load/save
│   ├── history.rs   # Watch history management
│   ├── cache.rs     # Search result caching
│   └── subscriptions.rs # Subscription management
├── ui/
│   ├── selector.rs  # Selector enum + factory
│   ├── fzf.rs       # fzf integration
│   └── dialoguer_selector.rs # Fallback prompts
└── utils/
    └── paths.rs     # XDG path utilities
```

### Rust Stack
- `clap` for CLI parsing (derive mode)
- `tokio` for async runtime
- `reqwest` for HTTP
- `serde` + `serde_json` for serialization
- `colored` for terminal colors
- `dialoguer` for interactive prompts
- `indicatif` for spinners (optional)
- `regex` for HTML parsing
- `sha2` for cache keys
- `chrono` for timestamps
- `dirs` for XDG paths
- `anyhow` + `thiserror` for error handling

## Installation

```bash
cargo install --path .
# Binary at ~/.cargo/bin/yt-chill
```

## State Machine

```
INIT → (menu) → SEARCH/HISTORY/FEED/SUBSCRIBE
SEARCH → (select video) → PLAY
HISTORY → (select video) → PLAY
PLAY → (stream/download/syncplay) → EXIT
```
