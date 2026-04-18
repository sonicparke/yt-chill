# yt-chill Correctness Pass

Scoped slice of [REFINEMENT.md](REFINEMENT.md) being implemented first. Eight targeted changes, no new features, no architectural refactor. Each change is independently testable.

Source plan: `.cursor/plans/yt-chill_correctness_pass_*.plan.md`.

---

## Todos

- [ ] Fix mpv exit-code check in [src/core/player.rs](src/core/player.rs) (drop `code == 4` special case, use `status.success()`)
- [ ] Remove `--copy-url` flag from `Cli` struct, `main.rs` branch, [README.md](README.md) and [PROMPT.md](PROMPT.md)
- [ ] Add `shellexpand` dep and expand `~` in `download_dir` during config load
- [ ] Add `#[serde(default)]` to `Config` struct and simplify `load_config` merge logic
- [ ] Migrate `subscriptions.txt` -> `subscriptions.json` with auto-migration (keep `.txt.bak`)
- [ ] Back up corrupt `history.json` and warn instead of silently wiping
- [ ] Extract shared `is_command_available` into `utils/process.rs` and run dependency check at startup
- [ ] Add `LazyLock<reqwest::Client>` with 15s timeout in [src/core/youtube.rs](src/core/youtube.rs)

---

## 1. Fix mpv exit-code handling — [src/core/player.rs](src/core/player.rs)

Current code at lines 63-71 treats only code `4` as a clean quit, but mpv returns `0` for a clean quit (including the `q` key) and non-zero for actual errors:

```rust
if !status.success() {
    // Don't treat user quit (q key) as an error
    if status.code() != Some(4) {
        return Err(YtChillError::Spawn(format!(
            "mpv exited with code: {:?}",
            status.code()
        )));
    }
}
```

Replace the inner branch with a straight `status.success()` check and drop the "code 4" comment. Clean quit is code 0; anything else is an error.

## 2. Remove `--copy-url` — [src/main.rs](src/main.rs), [README.md](README.md), [PROMPT.md](PROMPT.md)

- Delete the `copy_url` field from the `Cli` struct ([src/main.rs:59-60](src/main.rs)).
- Delete the early-return branch at [src/main.rs:312-317](src/main.rs).
- Remove the `--copy-url` row from the options table in [README.md](README.md) and the `yt-chill --copy-url` line from [PROMPT.md](PROMPT.md).

Rationale: flag doesn't do what its name suggests (it's a `println!`, not clipboard), it requires interactive selection first so it isn't scriptable, and at 0.1.0 there are no published users to break.

## 3. Expand `~` in `download_dir` — [src/storage/config.rs](src/storage/config.rs)

Add `shellexpand = "3"` to [Cargo.toml](Cargo.toml). In `load_config` ([src/storage/config.rs:31-38](src/storage/config.rs)), after resolving the string, run it through `shellexpand::tilde(&s).into_owned()` so `"~/Downloads"` becomes an absolute path before it reaches `yt-dlp`.

## 4. `Config` tolerant of missing/added fields — [src/types.rs](src/types.rs), [src/storage/config.rs](src/storage/config.rs)

- Add `#[serde(default)]` to the `Config` struct at [src/types.rs:84](src/types.rs). Each field falls back to its `Default` value if missing from the user's JSON.
- Simplify `load_config` at [src/storage/config.rs:11-41](src/storage/config.rs): drop the manual field-by-field merge. Deserialize once, then apply the two runtime-only touch-ups (download_dir fallback + tilde expansion from step 3).

## 5. Subscriptions -> JSON with backward-compat migration — [src/storage/subscriptions.rs](src/storage/subscriptions.rs)

- Change the on-disk file from `subscriptions.txt` to `subscriptions.json`, serialized via serde (`Vec<Subscription>`).
- In `load_subscriptions`:
  - If `subscriptions.json` exists, read it.
  - Else if `subscriptions.txt` exists, parse the old TSV, write out `subscriptions.json`, rename `.txt` -> `.txt.bak`.
  - Else return empty.
- Update `save_subscriptions` to write JSON.
- Update [src/utils/paths.rs:48-51](src/utils/paths.rs) `get_subscriptions_path` (and remove the duplicate in `subscriptions.rs`).

## 6. Surface history corruption — [src/storage/history.rs](src/storage/history.rs)

At [src/storage/history.rs:33-34](src/storage/history.rs) the code is:

```rust
let content = fs::read_to_string(&self.path).await?;
self.entries = serde_json::from_str(&content).unwrap_or_default();
```

Replace with: on parse failure, rename the file to `history.json.corrupt-<timestamp>`, print a warning to stderr via `eprintln!`, and start with an empty history. User keeps their old data for recovery; next save doesn't silently clobber it.

## 7. Startup dependency check — [src/main.rs](src/main.rs), new `src/utils/process.rs`

- Create `src/utils/process.rs` with a shared `async fn is_command_available(cmd: &str) -> bool`. Remove the duplicates in [src/core/player.rs:103-110](src/core/player.rs) and [src/core/downloader.rs:62-69](src/core/downloader.rs) and have them import the shared helper.
- In `main.rs`, after `ensure_app_dirs` but before state machine dispatch, check `mpv` is on PATH (always required) and `yt-dlp` if `cli.download` is set. On missing, print a one-line install hint (`brew install mpv yt-dlp` on macOS, `apt install mpv yt-dlp` on Linux via `cfg!(target_os)`) and exit non-zero.

## 8. HTTP timeout — [src/core/youtube.rs](src/core/youtube.rs)

At [src/core/youtube.rs:24](src/core/youtube.rs), `reqwest::Client::new()` is called per request with no timeout. Replace with a module-level `static CLIENT: LazyLock<reqwest::Client>` (stdlib `std::sync::LazyLock`, available since Rust 1.80) built with `.timeout(Duration::from_secs(15))` and the User-Agent baked in. Call sites just reference `&*CLIENT`.

---

## Verification

- `cargo build` + `cargo clippy -- -D warnings` clean.
- `cargo test` still passes (only one existing test, `test_build_search_url`).
- Manual:
  - `yt-chill "lofi beats"` streams and quits cleanly with code 0.
  - `yt-chill --history` on a freshly-corrupted `history.json` prints a warning and starts empty.
  - Moving an old `subscriptions.txt` into place and running `yt-chill -F` auto-migrates it.

---

## Out of scope (deferred to next pass)

Architectural items 11-18, feed rework (7-9), CI/release-profile (31-32), tests (33), and all other UX polish in [REFINEMENT.md](REFINEMENT.md).
