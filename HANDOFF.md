# yt-chill Handoff

Living tracker of which items from [REFINEMENT.md](REFINEMENT.md) are done, in-flight, or deferred. Update this file at the end of every session.

**Last updated:** 2026-04-18 (tooling + hygiene + UX + macOS-path slices)
**Current branch:** `main` (all pushed to `origin/main`)

---

## Status legend

- `[x]` done and in tree
- `[~]` in progress / partial
- `[ ]` not started
- `[-]` dropped / no longer planned

---

## Completed

### Session 2026-04-18 — tooling + hygiene + UX + macOS-path

Executed in four slices. Slice 1 was done in the parent agent; slices 2–4 ran in parallel as worktree subagents and were merged back sequentially with one small post-merge cleanup (the stale `#[allow(dead_code)]` markers slice 2 had left on the helpers slice 3 then wired up).

**Tooling pass** (items 31, 32, 33, commit `b586250`)

- [x] **31.** `.github/workflows/ci.yml` runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` on ubuntu-latest with `Swatinem/rust-cache`.
- [x] **32.** `[profile.release]` with `lto = true`, `codegen-units = 1`, `strip = true`.
- [x] **33.** 22 new unit tests (23 total). Covers `parse_search_results`, `parse_channel_results` (all three handle-shape branches), `parse_channel_videos_tab` (richGrid walk, empty-tab skip, reelItemRenderer filter, simpleText title fallback, metadata vs fallback author), `build_channel_videos_url` (including legacy `@UCxxx` rewrite), `History` round-trip (order, dedup, trimming, missing-file, corrupt-JSON backup), and `cache::is_expired` (extracted pure helper so TTL is testable without touching the filesystem or the clock).

**Hygiene pass** (items 13, 17, 34, commit `9eceef7` merged as `b31a0db`)

- [x] **13.** Deleted `CliOptions`, `AppContext` ([src/types.rs](src/types.rs)) and `ErrorCode` + `YtChillError::code()` ([src/error.rs](src/error.rs)) — verified zero usages across the repo. Removed module-level `#![allow(dead_code)]` from `src/types.rs`, `src/storage/cache.rs`, `src/storage/subscriptions.rs`. Kept `Channel`, `VideoFormat`, `YtChillError::{NoSelection, InvalidConfig}` with per-item allows (future-looking vocabulary).
- [x] **17.** `ytInitialData` regex now in `static YT_INITIAL_DATA_RE: LazyLock<Regex>` in [src/core/youtube.rs](src/core/youtube.rs) — no more per-call recompilation.
- [x] **34.** Bumped `colored` 2 -> 3 and `dirs` 5 -> 6 in [Cargo.toml](Cargo.toml). Zero call-site changes needed.

**UX polish pass** (items 20, 21, 22, 24, 25, 26, commit `62d75cc` merged as `b9806b6`'s parent chain)

- [x] **20.** `prompt_nonempty` helper in [src/main.rs](src/main.rs); `AppState::Search` and `AppState::Subscribe` now re-prompt on empty input with a yellow hint. Ctrl-C still bubbles via `?`.
- [x] **21.** `--list-subs` (prints handle + name) and `--unsubscribe` (selector-driven removal) flags; handled before the state machine.
- [x] **22.** `--clear-cache` and `--clear-history` flags wired to existing `clear_cache()` and `History::clear()` helpers.
- [x] **24.** Custom `tokio::spawn` + `\r\x1b[K` buffering animation replaced with `indicatif::ProgressBar::new_spinner()` (cyan, 80ms steady tick). The 6-second "🎵 Vibing..." transition is preserved.
- [x] **25.** [src/core/downloader.rs](src/core/downloader.rs) audio path now defaults to `--audio-format best` (native opus/m4a, no transcode). `DownloadOptions.format` still overrides.
- [x] **26.** `Config::default().editor` is now `std::env::var("EDITOR").unwrap_or_else(|_| "vi".into())`.

**macOS config path** (not originally in REFINEMENT, commit `001fc73` merged as `b9806b6`)

- [x] On macOS, `get_config_dir` / `get_cache_dir` now use `$HOME/.config` and `$HOME/.cache` (XDG-style) instead of `~/Library/Application Support` and `~/Library/Caches`. `XDG_CONFIG_HOME` / `XDG_CACHE_HOME` still honored. Linux unchanged. Windows still uses `dirs::*` unconditionally.
- Best-effort one-time migration on macOS: if legacy `~/Library/Application Support/yt-chill` or `~/Library/Caches/yt-chill` exist and the new dirs don't, they're moved over (rename with copy fallback for cross-filesystem). Migration failures are logged and swallowed.
- 5 new unit tests for `resolve_xdg_base` (pure, no env mutation). Total tests now 28.

### Session 2026-04-16 — feed-fix pass

Items 7, 8, 9 from the high-value slice.

- [x] **9.** `parse_channel_results` now reads `canonicalBaseUrl` (e.g. `/@Handle` or `/channel/UCxxxx`), stores the stripped path as the subscription handle — [src/core/youtube.rs](src/core/youtube.rs)
- [x] **8.** Real channel-videos parser: new `build_channel_videos_url` (hits `youtube.com/{handle}/videos`, with legacy `@UCxxxx` -> `channel/UCxxxx` rewrite for already-stored bad subs) and new `parse_channel_videos_tab` walking `twoColumnBrowseResultsRenderer -> tabs[] -> richGridRenderer.contents[] -> richItemRenderer.content.videoRenderer`. Author comes from `metadata.channelMetadataRenderer.title`. `fetch_channel_videos` returns `YouTubeParse` on empty instead of caching an empty list — [src/core/youtube.rs](src/core/youtube.rs)
- [x] **7.** Feed loop in `AppState::Feed` rewritten with `tokio::task::JoinSet`; per-channel errors are collected and printed as `warn: <name>: <err>` after all tasks finish — [src/main.rs](src/main.rs)

Ancillary:

- [x] Ran `cargo fmt` on the whole tree (prior session left lots of unformatted code). `cargo clippy -- -D warnings` and `cargo test` both pass.

Manual smoke test — done. Subscribed to a channel whose YouTube response omitted `canonicalBaseUrl`, so the handle stored as `channel/UC_JhYV43bqoR_P6z2aB80hA` (the fallback shape, not the broken `@UC...`). `--feed` loaded videos and played one; no `warn:` lines. Config path note: on macOS `dirs::config_dir()` is `~/Library/Application Support`, so subscriptions live at `~/Library/Application Support/yt-chill/subscriptions.json` (not `~/.config/yt-chill/`).

### Session 2026-04-16 — correctness pass

See [CORRECTNESS_PASS.md](CORRECTNESS_PASS.md) for the detailed plan that was executed.

- [x] **1.** Fix `mpv` exit-code check — [src/core/player.rs](src/core/player.rs)
- [x] **2.** Remove `--copy-url` flag — [src/main.rs](src/main.rs), [README.md](README.md), [PROMPT.md](PROMPT.md). Decision: feature dropped, not fixed.
- [x] **3.** Expand `~` in `download_dir` via `shellexpand` — [src/storage/config.rs](src/storage/config.rs), [Cargo.toml](Cargo.toml)
- [x] **4.** `#[serde(default)]` on `Config` + simplified `load_config` — [src/types.rs](src/types.rs), [src/storage/config.rs](src/storage/config.rs)
- [x] **5.** Subscriptions migrated to JSON with legacy TSV auto-migration (`.txt.bak`) — [src/storage/subscriptions.rs](src/storage/subscriptions.rs), [src/utils/paths.rs](src/utils/paths.rs)
- [x] **6.** Corrupt `history.json` backed up to `history.json.corrupt-<ts>` with stderr warning — [src/storage/history.rs](src/storage/history.rs)
- [x] **10.** Startup dependency check for `mpv` / `yt-dlp` / `syncplay` with platform-aware install hint — [src/main.rs](src/main.rs)
- [x] **38.** `reqwest::Client` held in `LazyLock` with 15s timeout and UA baked in — [src/core/youtube.rs](src/core/youtube.rs)

Ancillary (from clippy cleanup during verification):

- [x] Added `#[derive(Default)]` on `FzfSelector` and `DialoguerSelector` — [src/ui/fzf.rs](src/ui/fzf.rs), [src/ui/dialoguer_selector.rs](src/ui/dialoguer_selector.rs)
- [x] Extracted duplicated `is_command_available` to shared helper (covers item **11** partially) — new [src/utils/process.rs](src/utils/process.rs)
- [x] Removed dead duplicate `get_subscriptions_path` from `utils/paths.rs` (covers item **12**)

---

## Pending — high-value (recommended next)

Tooling, hygiene, UX, and macOS-path slices are all done. Remaining items are open-ended polish (14, 15, 18, 19, 23, 27, 28, 29, 35, 36, 37).

---

## Pending — architecture / DRY

- [~] **11.** `is_command_available` de-duplicated (done). Consider walking `$PATH` in Rust instead of spawning `which` (open)
- [x] **12.** `get_subscriptions_path` duplication resolved
- [x] **13.** Module-level `#![allow(dead_code)]` removed; `CliOptions` / `AppContext` / `ErrorCode` deleted; `clear_cache` / `History::clear` / `remove_subscription` wired into `--clear-cache` / `--clear-history` / `--unsubscribe` (session 2026-04-18)
- [ ] **14.** Flatten the theatrical state machine in [src/main.rs](src/main.rs), or make it real (post-play return to menu)
- [ ] **15.** Consider replacing the `Selector` enum + factory with `Box<dyn Selector>`
- [x] **16.** `reqwest::Client` shared (done as part of item 38)
- [x] **17.** `ytInitialData` regex wrapped in `LazyLock<Regex>` (session 2026-04-18)
- [ ] **18.** Move `"video:"` / `"channel:"` cache-key prefixing into typed cache helpers — [src/storage/cache.rs](src/storage/cache.rs)
- [ ] **19.** Flatten 8-10-deep `.get(...).and_then(...)` chains in `parse_search_results` / `parse_channel_results` (helper fn or `serde_json_path`)

---

## Pending — CLI / UX polish

- [x] **20.** Empty query re-prompts with a hint (session 2026-04-18)
- [x] **21.** `--list-subs` / `--unsubscribe` flags (session 2026-04-18)
- [x] **22.** `--clear-cache` / `--clear-history` flags (session 2026-04-18)
- [ ] **23.** Add `tracing` + `-v/--verbose` flag for debuggable YouTube parse failures
- [x] **24.** Buffering animation replaced with `indicatif` spinner (session 2026-04-18)
- [x] **25.** `--audio-format best` default (session 2026-04-18)
- [x] **26.** `editor` default now `$EDITOR -> vi` (session 2026-04-18)
- [ ] **27.** Implement or remove `SelectorType::Rofi` — [src/ui/selector.rs:47](src/ui/selector.rs)
- [ ] **28.** Reconcile `PlayerType::Syncplay` config vs `--syncplay` flag (pick one)
- [ ] **29.** Implement or remove `notify: bool` config
- [x] **30.** Covered by item 3 (tilde expansion)

---

## Pending — tooling / repo

- [x] **31.** `.github/workflows/ci.yml` (session 2026-04-18)
- [x] **32.** `[profile.release]` tweaks (session 2026-04-18)
- [x] **33.** Fixture-based tests (session 2026-04-18)
- [x] **34.** `colored` v3, `dirs` v6 (session 2026-04-18)
- [ ] **35.** (optional) `.cargo/config.toml` with cross-build rustflags
- [ ] **36.** Note crates.io publish status in [README.md](README.md)

---

## Pending — security (minor)

- [ ] **37.** Document `editor` config as a code-execution surface (low risk)
- [x] **38.** HTTP timeout added

---

## Deferred / dropped

- [-] **2.** `--copy-url` — removed, not fixed. If a clipboard feature is wanted later, add it as a first-class capability with `arboard`.

---

## Suggested next slice

Most of the high-value work is landed. Natural next batches:

- **Observability / debuggability**: 23 (tracing + `-v/--verbose`). Would make future YouTube-layout breakage much easier to diagnose.
- **Parser readability**: 18 + 19. Flatten the `.get(...).and_then(...)` chains and move `"video:"` / `"channel:"` cache-key prefixing into typed helpers. Purely internal.
- **State machine**: 14. Make the post-play loop actually loop back to the menu, or flatten the state-machine theatrics.
- **Config polish**: 27 + 28 + 29. Decide on Rofi / Syncplay / notify — either implement or delete.
- **Cross-build / publish**: 35 + 36.

Ask the user which they'd prefer before starting.

---

## Session workflow for future agents

1. Read this file first. Find the pending slice the user wants to work on.
2. Draft a plan in plan mode, cite specific file paths, confirm scope with the user before executing.
3. After execution: mark items `[x]` in this file, move completed items under a new `### Session YYYY-MM-DD` heading, and update the `Last updated` date at the top.
4. If you dropped or changed scope on an item, note it under `Deferred / dropped` with rationale.
