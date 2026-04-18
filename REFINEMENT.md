# yt-chill Refinement Pass

A grouped-by-impact review of the codebase as of the session on 2026-04-16. Captured so it survives context resets.

The current correctness slice (items 1-4, 5-6, 10, 38) is being implemented separately; see `.cursor/plans/yt-chill_correctness_pass_*.plan.md`. Everything else below is deferred.

---

## High-value fixes (correctness / things that probably bite users today)

1. **mpv exit-code check is wrong** — [src/core/player.rs:65](src/core/player.rs) treats only exit code `4` as a clean user quit, but `mpv` returns `0` when you press `q`. Most "successful" quits are either silently fine (because `status.success()` is true) or get reported as errors. Drop the `code != Some(4)` check; use `status.success()`.

2. **`--copy-url` doesn't copy** — [src/main.rs:313](src/main.rs) just `println!`s the URL. Either rename to `--print-url`, actually copy via `arboard`, or (recommended) remove the flag — it's niche and non-scriptable since it still forces interactive selection.

3. **Tilde isn't expanded in `download_dir`** — `cfg.download_dir` from the JSON config can be `"~/Downloads"`, and that string is passed straight to `yt-dlp`'s `-o` template at [src/core/downloader.rs:28](src/core/downloader.rs). `yt-dlp` does expand `~` itself, but the same value is used in `dirs::download_dir()` fallback logic that mixes literal and expanded forms. Expand once at config load with `shellexpand` or `dirs::home_dir()`.

4. **Config merge silently breaks on missing fields** — [src/storage/config.rs:11](src/storage/config.rs) does `serde_json::from_str::<Config>` directly, so any missing key in the user's `config.json` panics deserialization. The "merge with defaults" code below that line is also a manual field-by-field copy that you'll forget to update. Replace both with `#[serde(default)]` on each `Config` field (or on the struct).

5. **Subscriptions stored as raw TSV with no escaping** — [src/storage/subscriptions.rs:43](src/storage/subscriptions.rs). A channel name containing `\t` or `\n` corrupts the file. Since we already serialize JSON everywhere else, promote this to `subscriptions.json` and use serde.

6. **History parse errors are swallowed** — [src/storage/history.rs:34](src/storage/history.rs) does `serde_json::from_str(&content).unwrap_or_default()`. If the file is corrupted, history is silently wiped on next save. At minimum log a warning and back up the file.

7. **Feed channels are fetched serially** — [src/main.rs:219](src/main.rs) loops `for sub in &subs { youtube::fetch_channel_videos(...).await }`. With 10 subs, you wait ~10xlatency. Use `futures::future::join_all` or `tokio::task::JoinSet`. Errors are also dropped silently (`Err(_) => {}`) — at least print which channels failed.

8. **The "feed" isn't really a feed** — `fetch_channel_videos` ([src/core/youtube.rs:251](src/core/youtube.rs)) builds a *search* query of `"<handle> "` and reuses the search-results parser. That gives a fuzzy keyword match across YouTube, not the channel's recent uploads. Hit `https://www.youtube.com/@handle/videos` and parse `tabRenderer -> richGridRenderer` instead.

9. **Channel handle parsing is incorrect** — [src/core/youtube.rs:213-224](src/core/youtube.rs) builds the handle as `@<channelId>` (e.g. `@UCxxxx`), but real YouTube handles are distinct from channel IDs. The `subscriberCountText` guard is also a no-op (`.and_then(|_| c.get("channelId"))`). Read `c["subscriberCountText"]` for display, but pull the handle from `c["navigationEndpoint"]["browseEndpoint"]["canonicalBaseUrl"]` (e.g. `/@somehandle`).

10. **Dependency check happens too late** — `which mpv` / `which yt-dlp` runs only at the moment of playback (`is_command_available` in player.rs and downloader.rs). The user can search, pick a video, and *then* be told mpv isn't installed. Check once at startup and print a friendly install hint.

---

## Architecture / DRY

11. **`is_command_available` is duplicated** in [src/core/player.rs:103](src/core/player.rs) and [src/core/downloader.rs:62](src/core/downloader.rs). Move to `src/utils/process.rs` and consider a Rust-native check (walk `$PATH`) instead of spawning `which`.

12. **`get_subscriptions_path` is duplicated** in both [src/utils/paths.rs:49](src/utils/paths.rs) and [src/storage/subscriptions.rs:11](src/storage/subscriptions.rs) (with different return types). Pick one.

13. **`#![allow(dead_code)]` hides real cruft** — present at the top of `types.rs`, `cache.rs`, `subscriptions.rs`, plus targeted `#[allow(dead_code)]`s elsewhere. Actually-unused items: `CliOptions`, `AppContext`, `ErrorCode`, `clear_cache`, `History::clear`, `remove_subscription`, `save_config`. Either wire them into the CLI (`--clear-cache`, `--clear-history`, `--unsubscribe`) or delete them.

14. **State machine is theatrical** — [src/main.rs](src/main.rs) declares an `AppState` enum and a `while state != Exit` loop, but every branch sets `state = Exit` (or `Play` then `Exit`). It's effectively a `match` that runs once. Either flatten to straight-line code, or make it real (after Play, return to History/Search instead of exiting).

15. **`Selector` enum + factory could just be `Box<dyn Selector>`** — current dispatch through an enum requires touching three files to add Rofi. Trait object would localize each backend.

16. **Reqwest client recreated per request** — `fetch_youtube_html` ([src/core/youtube.rs:24](src/core/youtube.rs)) calls `reqwest::Client::new()` each time. Hold one in a `OnceLock<Client>` so the connection pool is reused.

17. **`extract_yt_initial_data` regex compiled per call** and uses `.expect("Invalid regex")`. Wrap in `static RE: LazyLock<Regex>`.

18. **Cache key namespacing leaks into callers** — [src/storage/cache.rs](src/storage/cache.rs) `get_cache_key` is just SHA256, and callers manually prefix `"video:"` / `"channel:"`. Move that prefix logic into typed cache helpers (`get_search_cache(query)`, `get_channel_cache(handle)`).

19. **JSON parsing chains are 8-10 levels deep** — `parse_search_results` and `parse_channel_results` in [src/core/youtube.rs](src/core/youtube.rs). Pull a small `fn pluck(v: &Value, path: &[&str]) -> Option<&Value>` helper, or use `serde_json_path` to flatten them. Also `c.get(0)` should be `.as_array().and_then(|a| a.first())` (clippy lint).

---

## CLI / UX polish

20. **Empty query -> silent exit** — both the `Search` path ([src/main.rs:148](src/main.rs)) and `Subscribe` path ([src/main.rs:262](src/main.rs)) treat empty input as "user quit." Re-prompt or show "Query can't be empty" instead.

21. **No `--list-subs` / `--unsubscribe`** — you can `--subscribe`, but no way to inspect or remove via the CLI; user has to hand-edit `subscriptions.txt`.

22. **No `--clear-cache` / `--clear-history`** even though both functions exist.

23. **No `--verbose` / no logging** — when YouTube changes their HTML and parsing breaks, the user gets a one-line `Error:` with no context. Add `tracing` + `tracing-subscriber` and a `-v` flag.

24. **Buffering animation can race with mpv output** — [src/core/player.rs:37-48](src/core/player.rs) prints `"Convincing YouTube..."` then 6s later overwrites with `\r\x1b[K`. With `--really-quiet` it's mostly fine, but consider doing the spinner with `indicatif` (already a dep) so it self-clears, or skip the message and let mpv start.

25. **Forced MP3 transcoding** — [src/core/downloader.rs:18](src/core/downloader.rs) always re-encodes audio to MP3, which is slow and lossy. Default to `--audio-format best` (keeps native opus/m4a) and let users opt into mp3 via config.

26. **Hardcoded `nvim` editor default** — `Config::default()` ([src/types.rs:111](src/types.rs)) sets `editor: "nvim"`. Most users won't have it. Default to `$EDITOR` env var, then `vi`, then `nvim`.

27. **`SelectorType::Rofi` is documented but not implemented** — [src/ui/selector.rs:47](src/ui/selector.rs) silently falls back to dialoguer. Either implement or remove the variant.

28. **`PlayerType::Syncplay`** is in the config schema, but syncplay is actually picked via the `--syncplay` flag in [src/main.rs:322](src/main.rs). Config field never read. Pick one mechanism.

29. **`notify: bool` config is unused** — referenced in PROMPT.md and the README example, but no notification code exists.

30. **`download_dir` README example is `"~/Downloads"`** but [src/storage/config.rs:32](src/storage/config.rs) only handles the empty-string case for fallback. See item 3.

---

## Tooling / Repo

31. **No CI** — no `.github/workflows/`. A 30-line action running `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` on push would catch most of the above on any future change.

32. **No release profile tweaks** — for a `cargo install`-distributed binary, add to [Cargo.toml](Cargo.toml):

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
```

Will roughly halve binary size and noticeably improve startup.

33. **One test** — `test_build_search_url` is the only test. Worth adding fixture-based tests for `parse_search_results` (drop a real `ytInitialData` JSON into `tests/fixtures/`), `History` round-trip, and `cache` TTL expiry.

34. **`Cargo.toml` versions** — `colored = "2"` has a v3; `dirs = "5"` has a v6. `dialoguer = "0.11"` and `thiserror = "2"` are current. `edition = "2024"` is great.

35. **No `.cargo/config.toml`** with target-specific rustflags; not required but useful for cross-builds.

36. **`README.md` says `cargo install --path .`** but `Cargo.toml`'s `repository` is `https://github.com/sonicparke/yt-chill-rs`. Once published, `cargo install yt-chill` would also work. Worth noting it's not on crates.io.

---

## Security (minor)

37. **`editor` config value is exec'd as a process** — a malicious `config.json` in `~/.config/yt-chill/` could point `editor` at anything. Low risk (attacker already has filesystem access at that point), but worth documenting.

38. **No request timeout** — `reqwest::Client::new()` ([src/core/youtube.rs:24](src/core/youtube.rs)) has no timeout. A slow YouTube response hangs forever. Add `.timeout(Duration::from_secs(15))`.

---

## Suggested order

1. Fix items 1-4 (correctness): mpv exit code, copy-url, tilde, config defaults — all small, high-value.
2. Items 7, 8, 9 (the feed is the most broken feature; if it doesn't work, remove it or fix it properly).
3. Items 11-18 (architecture cleanup pass — half a day's work, makes the next change easier).
4. Items 31, 32, 33 (CI + release profile + tests — protects everything else).
5. Items 19-30 (polish).
