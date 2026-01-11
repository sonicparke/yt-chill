# yt-chill: Build Prompt

> This prompt describes how to build yt-chill from scratch in Rust.

## Overview

Build a command-line YouTube audio player in Rust called `yt-chill`. It should be distraction-free, audio-first, and installable globally via `cargo install`.

## Core Behavior

- **Audio-only by default** — streams audio, no video
- **Streaming by default** — no download unless `-d` flag
- **No menus after selection** — pick a video, it plays immediately
- **Keyboard controls work** — space to pause, q to quit, arrows to seek

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
```

## Technical Requirements

### YouTube Integration
- Scrape YouTube HTML directly (no API key)
- Parse `ytInitialData` JSON from response
- Cache search results with TTL

### Dependencies
- `mpv` for playback
- `yt-dlp` for downloads
- `fzf` (optional) for better selection UI, fallback to built-in prompts

### User Experience
- Show snarky buffering message: `⏳ Convincing YouTube to share... 🙄`
- After 6 seconds show: `🎵 Vibing... Sit back and chill. (space=pause, q=quit)`
- On exit show: `👋 Thanks for chilling.`
- Use colored terminal output
- Support fzf for video selection, fall back to dialoguer

### Storage
- Config at `~/.config/yt-chill/config.json` (XDG compliant)
- History at `~/.cache/yt-chill/history.json`
- Respect `XDG_CONFIG_HOME` and `XDG_CACHE_HOME`

### Rust Stack
- `clap` for CLI parsing
- `tokio` for async runtime
- `reqwest` for HTTP
- `serde` + `serde_json` for serialization
- `colored` for terminal colors
- `dialoguer` for interactive prompts
- `indicatif` for spinners (if needed)

## Installation

```bash
cargo install --path .
# Binary at ~/.cargo/bin/yt-chill
```
