# yt-chill Handoff

Living tracker of which items from [REFINEMENT.md](REFINEMENT.md) are done, in-flight, or deferred. Update this file at the end of every session.

**Last updated:** 2026-04-20 (TUI polish — menu resume + copy)
**Current branch:** `main` (all pushed to `origin/main`)

---

## Status legend

- `[x]` done and in tree
- `[~]` in progress / partial
- `[ ]` not started
- `[-]` dropped / no longer planned

---

## Completed

### Session 2026-04-20 — TUI polish (orchestration prompt)

Executed in-repo (single workspace) per [TUI_POLISH_AUTONOMOUS_ORCHESTRATION_PROMPT.md](TUI_POLISH_AUTONOMOUS_ORCHESTRATION_PROMPT.md). No separate git worktrees; **T4 skipped** (playback copy already in good shape from prior pass).

**T1 — [`src/main.rs`](../src/main.rs)**

- `started_from_main_menu`: plain `yt-chill` sessions return to the main menu after cancel, empty search results, recoverable search errors, empty history/feed, or backing out of a video/history/feed picker; one-shot CLI (`--history`, `--feed`, `--subscribe`, or a query on argv) still exits on those paths.
- Clearer empty-state and feed-failure copy; subscribe flow handles empty channel results and optional “Subscribe cancelled.” on Esc.
- Selector prompts aligned (“Choose a video / from history / from feed / a channel”).

**T2 — [`src/ui/dialoguer_selector.rs`](../src/ui/dialoguer_selector.rs), [`src/utils/flow.rs`](../src/utils/flow.rs)**

- `next_state_after_skipped_flow` + unit tests; dialoguer `Select` uses `.report(false)` and a short cancel/confirm comment.

**T3 — [`src/ui/fzf.rs`](../src/ui/fzf.rs)**

- Explicit `--no-multi` for consistent single-select behavior.

**Verification:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — **54** unit tests.

### Session 2026-04-19 — Test expansion + TUI polish

Executed in one workspace per [TEST_EXPANSION_AUTONOMOUS_ORCHESTRATION_PROMPT.md](TEST_EXPANSION_AUTONOMOUS_ORCHESTRATION_PROMPT.md) and [TUI_POLISH_AUTONOMOUS_ORCHESTRATION_PROMPT.md](TUI_POLISH_AUTONOMOUS_ORCHESTRATION_PROMPT.md) (no separate worktrees).

**Test expansion**

- **S1:** Extra unit coverage in [`src/storage/cache.rs`](../src/storage/cache.rs), [`src/storage/config.rs`](../src/storage/config.rs), [`src/utils/process.rs`](../src/utils/process.rs) (PATH trimming, non-executable files on Unix, Windows `PATH` separator test).
- **S2:** Parser/tracing regressions in [`src/core/youtube.rs`](../src/core/youtube.rs): JSON parse failure logs, skip video rows without `videoId`, skip channel rows with no handle.
- **S3:** [`src/utils/flow.rs`](../src/utils/flow.rs) — `next_state_after_play` extracted and covered by tests; [`src/main.rs`](../src/main.rs) uses it after playback.

**TUI polish**

- **T1:** Clearer menu labels, prompts, and status copy in [`src/main.rs`](../src/main.rs) (main menu, search/history/feed/subscribe flows, maintenance messages).
- **T2:** Dialoguer selector left on the stable `Select` API (dialoguer 0.11 has no `with_page_size`); no behavioral regression.
- **T3:** [`src/ui/fzf.rs`](../src/ui/fzf.rs) — aligned prompt (`›`), explicit `--layout reverse`, `--info inline`.
- **T4:** [`src/core/player.rs`](../src/core/player.rs) — calmer “Connecting…”, playback hint, and exit line.

**Verification:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — **52** unit tests (was 40).

### Session 2026-04-18 — Waves A–D (HANDOFF 11, 14, 15, 18, 19, 23, 27–29, 35–37)

Orchestration prompt `WAVES_AD_AUTONOMOUS_ORCHESTRATION_PROMPT.md` executed in-repo (single workspace).

**Wave A — cache, PATH, docs**

- [x] **18 + 19.** Typed cache keys [`cache_key_video_search`](src/storage/cache.rs) / [`cache_key_channel_videos`](src/storage/cache.rs); [`youtube.rs`](src/core/youtube.rs) uses them. Search parsers flattened via `search_item_section_contents`, `parse_video_renderer`, `parse_channel_renderer`.
- [x] **11.** [`resolve_executable_in_path`](src/utils/process.rs) + PATH walk; [`is_command_available`](src/utils/process.rs) is sync (no `which`). [`FzfSelector::is_available`](src/ui/fzf.rs) uses the same helper.
- [x] **35.** Optional `.cargo/config.toml` with cross-target `rustflags` **not** added (would risk default ubuntu CI); treated as completed doc-only per orchestration fallback.
- [x] **36 + 37.** [README.md](README.md): crates.io / install-from-git; Syncplay policy. [SECURITY.md](SECURITY.md): `editor` execution surface.

**Wave B — tracing**

- [x] **23.** `tracing` + `tracing-subscriber` + `-v` / `--verbose`; [`extract_yt_initial_data`](src/core/youtube.rs) emits `tracing::debug!` on regex/JSON parse issues; [`tracing_test`](src/core/youtube.rs) regression for regex miss.

**Wave C — config cleanup**

- [x] **29.** Removed unused `notify` from [`Config`](src/types.rs); serde ignores legacy JSON (unit test).
- [x] **27.** Removed `SelectorType::Rofi`; [`load_config`](src/storage/config.rs) maps `"rofi"` / `"Rofi"` → dialoguer before typed parse (tests).
- [x] **28.** [`use_syncplay`](src/utils/playback.rs) = `cli_flag || player == Syncplay`; wired in [`main.rs`](src/main.rs) for deps + playback path; README documented.

**Wave D — loop + selector**

- [x] **14.** After a **successful** `AppState::Play` (stream / download / syncplay), state returns to `AppState::Init` instead of exiting; failures still exit.
- [x] **15.** [`DynSelector`](src/ui/selector.rs) (`Box<dyn MenuSelector>`) replaces the public `Selector` enum; fzf/dialoguer adapters implement `select_index`.

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
- [x] Extracted duplicated `is_command_available` to shared helper — new [src/utils/process.rs](src/utils/process.rs) (item **11** completed later same day: PATH walk, no `which`)
- [x] Removed dead duplicate `get_subscriptions_path` from `utils/paths.rs` (covers item **12**)

---

## Pending — high-value (recommended next)

No open REFINEMENT items tracked here; see [REFINEMENT.md](REFINEMENT.md) for any new backlog.

---

## Pending — architecture / DRY

- [x] **11.** `is_command_available` uses pure PATH walk (session 2026-04-18 Waves A–D)
- [x] **12.** `get_subscriptions_path` duplication resolved
- [x] **13.** Module-level `#![allow(dead_code)]` removed; `CliOptions` / `AppContext` / `ErrorCode` deleted; `clear_cache` / `History::clear` / `remove_subscription` wired into `--clear-cache` / `--clear-history` / `--unsubscribe` (session 2026-04-18)
- [x] **14.** Post-play return to main menu on success (session 2026-04-18 Waves A–D)
- [x] **15.** `DynSelector` + `Box<dyn MenuSelector>` (session 2026-04-18 Waves A–D)
- [x] **16.** `reqwest::Client` shared (done as part of item 38)
- [x] **17.** `ytInitialData` regex wrapped in `LazyLock<Regex>` (session 2026-04-18)
- [x] **18.** Typed cache key helpers (session 2026-04-18 Waves A–D)
- [x] **19.** Flatter search parsers via shared section helper + per-renderer parsers (session 2026-04-18 Waves A–D)

---

## Pending — CLI / UX polish

- [x] **20.** Empty query re-prompts with a hint (session 2026-04-18)
- [x] **21.** `--list-subs` / `--unsubscribe` flags (session 2026-04-18)
- [x] **22.** `--clear-cache` / `--clear-history` flags (session 2026-04-18)
- [x] **23.** `tracing` + `-v` / `--verbose` (session 2026-04-18 Waves A–D)
- [x] **24.** Buffering animation replaced with `indicatif` spinner (session 2026-04-18)
- [x] **25.** `--audio-format best` default (session 2026-04-18)
- [x] **26.** `editor` default now `$EDITOR -> vi` (session 2026-04-18)
- [x] **27.** `Rofi` removed; legacy `"rofi"` config mapped to dialoguer (session 2026-04-18 Waves A–D)
- [x] **28.** `use_syncplay(cli, cfg.player)` policy (session 2026-04-18 Waves A–D)
- [x] **29.** `notify` removed from `Config`; legacy JSON ignored (session 2026-04-18 Waves A–D)
- [x] **30.** Covered by item 3 (tilde expansion)

---

## Pending — tooling / repo

- [x] **31.** `.github/workflows/ci.yml` (session 2026-04-18)
- [x] **32.** `[profile.release]` tweaks (session 2026-04-18)
- [x] **33.** Fixture-based tests (session 2026-04-18)
- [x] **34.** `colored` v3, `dirs` v6 (session 2026-04-18)
- [x] **35.** Doc-only: optional rustflags `.cargo/config.toml` skipped to protect default CI (session 2026-04-18 Waves A–D)
- [x] **36.** README install / crates.io note (session 2026-04-18 Waves A–D)

---

## Pending — security (minor)

- [x] **37.** [SECURITY.md](SECURITY.md) + README pointer (session 2026-04-18 Waves A–D)
- [x] **38.** HTTP timeout added

---

## Deferred / dropped

- [-] **2.** `--copy-url` — removed, not fixed. If a clipboard feature is wanted later, add it as a first-class capability with `arboard`.

---

## Suggested next slice

REFINEMENT tracker items above are closed through **38**. Next work is new scope from [REFINEMENT.md](REFINEMENT.md) or product direction.

---

## Session workflow for future agents

1. Read this file first. Find the pending slice the user wants to work on.
2. Draft a plan in plan mode, cite specific file paths, confirm scope with the user before executing.
3. After execution: mark items `[x]` in this file, move completed items under a new `### Session YYYY-MM-DD` heading, and update the `Last updated` date at the top.
4. If you dropped or changed scope on an item, note it under `Deferred / dropped` with rationale.
5. **Subagent + merge workflow (TDD slices):** Run the slice as a Cursor Task with `subagent_type: best-of-n-runner` (or your usual worktree agent). When it returns, merge the branch it reports (e.g. `cursor/a1-youtube-cache-internals-tdd`) into `main` in the **parent** session, run the same gates (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`), update this `HANDOFF.md`, push, then start the **next** slice (e.g. **A2** PATH / `which`) using the **same TDD block** in that slice’s Task prompt.

---

## Appendix — Task prompt: A1 (`HANDOFF` 18 + 19, TDD)

Copy everything inside the fence into a Cursor Task (e.g. `best-of-n-runner`). The closing **Orchestration** paragraph is for the **parent** agent after the subagent finishes; you may leave it in the Task so the subagent sees merge is out of scope for it.

```text
You are implementing HANDOFF.md items 18 + 19 for the yt-chill Rust CLI.

## Repo and isolation

- Primary repo: /Users/bradmcalister/DEV/yt-chill
- Base: checkout a NEW worktree from `origin/main` (use latest SHA; run `git fetch origin && git rev-parse origin/main` first).
- Worktree path (example): /Users/bradmcalister/DEV/yt-chill-wt/a1-youtube-cache-internals
- Branch name: `cursor/a1-youtube-cache-internals-tdd`
- DO NOT push. DO NOT merge to main. One PR-ready branch.

## Scope (exact)

### Item 18 — typed cache key helpers

Today `src/core/youtube.rs` builds cache keys with string prefixes inline:

- `fetch_search_results`: `get_cache_key(&format!("video:{}:{}", query, limit))`
- `fetch_channel_videos`: `get_cache_key(&format!("channel:{}:{}", channel_handle, limit))`

Move prefix + composition into `src/storage/cache.rs` as small, documented helpers, e.g.:

- `pub fn cache_key_video_search(query: &str, limit: usize) -> String` → must produce the **same** final key string as `get_cache_key(&format!("video:{}:{}", query, limit))` today (byte-for-byte identical SHA input).
- `pub fn cache_key_channel_videos(handle: &str, limit: usize) -> String` → same as current `get_cache_key(&format!("channel:{}:{}", channel_handle, limit))`.

Keep `get_cache_key` as the low-level hasher OR fold hashing inside the new helpers — your choice, but **existing on-disk cache files must remain valid** (same key strings as before for the same inputs).

Update `youtube.rs` call sites to use the new helpers only.

### Item 19 — flatten parser chains

In `src/core/youtube.rs`, reduce depth of `.get(...).and_then(...)` in `parse_search_results` and `parse_channel_results` by extracting small pure helpers (private `fn` or `mod parse` with helpers). **No behavior change**: same JSON in → same `Vec<Video>` / channel structs out.

`parse_channel_videos_tab` is optional to touch only if you can do so without risk; priority is search + channel **search** parsers.

## TDD (mandatory order)

1. **RED — tests first**
   - In `src/storage/cache.rs` `#[cfg(test)]` (or a focused test module), add tests that lock the **contract** for the new helpers:
     - For several `(query, limit)` pairs, `cache_key_video_search(q, n)` must equal `get_cache_key(&format!("video:{}:{}", q, n))`.
     - For several `(handle, limit)` pairs, `cache_key_channel_videos(h, n)` must equal `get_cache_key(&format!("channel:{}:{}", h, n))`.
   - Add 1–2 **parser regression tests** in `src/core/youtube.rs` `#[cfg(test)]` if the existing suite does not already cover a branch you will refactor (e.g. a minimal extra fixture for a shallow path you split out). New tests must **fail** before you refactor (e.g. temporarily wrong helper return, or assert on a helper you have not added yet — pick a honest RED).
   - Run `cargo test` and **capture** that new tests fail.

2. **GREEN**
   - Implement helpers + wire `youtube.rs`; refactor parsers until all tests pass.

3. **REFACTOR (optional)**
   - Internal cleanup only; tests stay green.

4. **Gate**
   - `cargo fmt`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`

## Commits (preferred)

Two commits on your branch:

1. `test: lock cache key contracts before typed helpers`
2. `refactor: typed cache keys and flatter YouTube parsers`

If you truly must use one commit, state TDD in the message and keep tests logically first in the diff.

## Files in scope

- `src/storage/cache.rs`
- `src/core/youtube.rs`
- `Cargo.toml` / `Cargo.lock` only if you add a dev-dep (avoid unless necessary)

## Out of scope

- Item 23 tracing, items 27–29, HANDOFF anything else
- Changing TTL, cache file layout, or HTTP behavior

## Success criteria

- All new tests described above exist and pass.
- `cache_key_*` helpers are used from `youtube.rs`; no remaining `format!("video:{}:{}"` / `format!("channel:{}:{}"` for cache keys in `youtube.rs`.
- Parser outputs unchanged for existing tests; `cargo clippy --all-targets -- -D warnings` clean.

## Report back to parent

Return: worktree path, branch name, commit SHAs (1 or 2), list of new test names, and any file touched.

---

Orchestration (parent agent — subagent does NOT do this): You can run this as a Task with `subagent_type: best-of-n-runner` (or your usual worktree agent). When it returns, merge `cursor/a1-youtube-cache-internals-tdd` into `main` in the parent session, run the same gates, update `HANDOFF.md`, then move to **A2** (PATH / `which`) with the same TDD block.
```
