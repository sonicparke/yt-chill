# Autonomous orchestration prompt (Waves A–D)

**Purpose:** Paste the entire block below into a **single long-running Cursor agent / Composer session** that can run terminals and subagents **without** asking you to approve each command. Use a profile with **auto-run / YOLO** (or approve-all) for `Shell` and `Task`, and grant **`all`** (or `git_write` + `full_network`) so `git`, `cargo`, and worktrees are not sandbox-blocked.

**Repo root (edit if yours differs):** `REPO=/Users/bradmcalister/DEV/yt-chill`  
**Worktrees parent:** `WT_PARENT=/Users/bradmcalister/DEV/yt-chill-wt`  
**Remote:** `origin`  
**Integration branch:** always merge into `main`, then `git push origin main`.

---

## BEGIN PASTE BELOW THIS LINE

You are the **parent orchestrator** for the `yt-chill` Rust CLI. Your job is to **fully implement HANDOFF.md waves A through D** using **git worktrees**, **subagents** (`Task` with `subagent_type: best-of-n-runner` where available), **TDD** on every code slice, **merge + conflict resolution**, **HANDOFF.md updates**, and **push to `origin/main`**. The user will **not** be available to approve commands—use the broadest permissions you can (`all` on `Shell` / `Task`) and do not stop for optional confirmations.

### Global rules

1. **Before anything:** `cd "$REPO" && git fetch origin && git checkout main && git pull origin main`. Work from a clean tree (stash or commit only if unavoidable; prefer clean).
2. **After every merge into `main`:**  
   `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`  
   If anything fails, **fix on `main`** (minimal fix), commit, push, then continue.
3. **Subagent contract:** Each slice runs in its **own worktree** and **own branch** `cursor/<slice>`. Subagents: **do not push** from worktrees; return **branch name + commit SHAs + files touched + new test names**.
4. **TDD block (mandatory for every Rust slice):**  
   **RED** → add failing tests; `cargo test` shows new failures.  
   **GREEN** → implement until all tests pass.  
   **REFACTOR** (optional) → internal cleanup; tests stay green.  
   **Gate** → `cargo fmt` + `cargo clippy --all-targets -- -D warnings` + `cargo test`.  
   Prefer **two commits** per slice branch: `test: …` then `feat|refactor: …`.
5. **Merging:** After subagent returns, merge with `git merge --no-ff <branch> -m "Merge <branch>: <short summary>"`. Resolve conflicts manually; **never** drop tests or loosen clippy to “make it compile” without equivalent behavior.
6. **HANDOFF.md:** After each wave completes on `main`, update `[ ]` → `[x]`, add a `### Session YYYY-MM-DD` bullet for that wave, bump **Last updated**, commit on `main`, push.
7. **Cleanup:** Remove worktrees when done: `git worktree remove <path>` and `git branch -d <branch>` for merged topic branches if not needed.

### TDD snippet (embed in every code subagent Task prompt)

```text
TDD (mandatory): (1) RED — add failing tests; cargo test must show new failures.
(2) GREEN — minimal implementation until all tests pass.
(3) REFACTOR — optional; tests stay green.
(4) cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test.
Prefer two commits: test: … then feat/refactor: …
Do not push from the worktree. Report branch name, SHAs, new test names, files touched.
```

---

### Wave A — run **three subagents in parallel** (separate `Task` calls in one message if your UI allows; else run sequentially but start from the same `origin/main` SHA **before** any merges, then merge in the order A1 → A2 → A3).

**Prepare worktrees parent:** `mkdir -p "$WT_PARENT"`

#### A1 — HANDOFF **18 + 19** (typed cache keys + flatter YouTube parsers)

- **Branch:** `cursor/a1-youtube-cache-internals-tdd`  
- **Worktree:** `"$WT_PARENT/a1-youtube-cache-internals"`  
- **Base:** `git worktree add -b cursor/a1-youtube-cache-internals-tdd "$WT_PARENT/a1-youtube-cache-internals" origin/main`  
- **Spec:** Use the full prompt in **HANDOFF.md → Appendix “Task prompt: A1 (18 + 19, TDD)”** (verbatim), plus the **TDD snippet** above.  
- **Files:** `src/storage/cache.rs`, `src/core/youtube.rs` (+ `Cargo.toml` only if strictly needed).

#### A2 — HANDOFF **11** (PATH resolution without spawning `which`)

- **Branch:** `cursor/a2-path-resolve-no-which-tdd`  
- **Worktree:** `"$WT_PARENT/a2-path-resolve-no-which"`  
- **Base:** same `origin/main` **SHA as A1’s base** (the commit before any Wave A merges).  
- **Spec:**  
  - **RED:** Table-driven unit tests for a **pure** helper, e.g. `resolve_executable_in_path(executable: &str, path_entries: &[String]) -> Option<PathBuf>` (or split platform logic with `#[cfg]`). No real filesystem required for core cases; optional integration test behind `#[cfg(unix)]` if needed.  
  - **GREEN:** Refactor `src/utils/process.rs` (`is_command_available` / related) to use PATH walk; remove `which` / shell spawning if present.  
  - **Gate:** fmt + clippy `-D warnings` + test.

#### A3 — HANDOFF **35 + 36 + 37** (docs / tooling / security note)

- **Branch:** `cursor/a3-docs-tooling-security`  
- **Worktree:** `"$WT_PARENT/a3-docs-tooling-security"`  
- **Base:** same pre–Wave-A `origin/main` SHA.  
- **Spec:**  
  - **35:** Add optional `.cargo/config.toml` with sensible `rustflags` for cross-target builds **only if** it does not break default `cargo build` / CI on ubuntu. If risky, add a short comment in HANDOFF that 35 is “doc-only optional” and skip file—prefer a **safe** minimal config.  
  - **36:** README: crates.io / publish status / “install from git” story.  
  - **37:** README (or `SECURITY.md`): `editor` runs arbitrary commands—low practical risk, document clearly.  
  - **TDD:** N/A for prose; still run **fmt + clippy + test** on branch (must stay green). One commit acceptable.

**Merge Wave A (order):** checkout `main` → merge **A1** → gates → push → merge **A2** → gates → push → merge **A3** → gates → push → update **HANDOFF** for items **11, 18, 19, 35, 36, 37** as completed → commit + push HANDOFF.

---

### Wave B — HANDOFF **23** (`tracing` + `-v` / `--verbose`)

- **Branch:** `cursor/b1-tracing-verbose-tdd`  
- **Worktree:** `"$WT_PARENT/b1-tracing-verbose"`  
- **Base:** `origin/main` **after Wave A fully merged and pushed**.  
- **TDD snippet:** required.  
- **RED:** Tests using a `tracing` capture subscriber (or `tracing-test` if you add the dev-dep) that assert: with verbose on, a **simulated** or **injected** parse failure path emits a diagnostic event/span containing agreed markers; with verbose off, those diagnostics are absent (or only stable minimal noise).  
- **GREEN:** Add `tracing`, `tracing-subscriber` (keep deps minimal), plumb `--verbose` / `-v` in `src/main.rs`, initialize subscriber once, instrument `src/core/youtube.rs` (and any other high-signal sites you choose) without spamming default mode.  
- **Gate + merge + push + HANDOFF** `[x]` for **23**.

---

### Wave C — HANDOFF **27 + 28 + 29** (Rofi / Syncplay / notify) — **unattended policy** (do not ask the user)

Implement exactly this policy so the run always terminates:

1. **29 `notify`:** Field is **unused** in code today. **Remove** `notify` from `Config` in `src/types.rs`, from `Default`, and from README example. Serde ignores unknown fields in old JSON by default—verify deserialize still works with a **unit test** loading a JSON snippet that still contains `"notify": true`.

2. **27 Rofi:** **Remove** `SelectorType::Rofi` from the public enum. **Backward compatibility:** when loading `config.json`, if deserialization fails due to `"selector":"rofi"`, **or** proactively: deserialize `serde_json::Value` and map `selector` string `"rofi"` / `"Rofi"` → `"dialoguer"` before typed parse, **or** implement a custom Deserialize—choose the smallest approach. **RED:** test that config JSON with `"selector":"rofi"` still loads and yields dialoguer behavior. **GREEN:** remove variant + fix `create_selector` / all matches.

3. **28 Syncplay:** **Single rule:** `use_syncplay(cli_flag: bool, cfg_player: PlayerType) -> bool` is **`cli_flag || cfg_player == PlayerType::Syncplay`**. Use this everywhere playback chooses Syncplay vs mpv (`src/main.rs` and any helper). **RED:** unit tests for `use_syncplay` truth table. **GREEN:** wire; **document** in README: CLI flag **or** config `player: syncplay` enables Syncplay path.

**Gate + merge + push + HANDOFF** `[x]` for **27, 28, 29**.

---

### Wave D — large refactors (sequential; each after previous is on `main`)

#### D1 — HANDOFF **14** (state machine / post-play loop)

- **Branch:** `cursor/d1-state-machine-loop-tdd`  
- **Worktree:** `"$WT_PARENT/d1-state-machine-loop"`  
- **Base:** `origin/main` after Wave C.  
- **TDD:** **RED** first with **pure** extracted transition helpers or a small harness (avoid full TUI e2e if impractical). Tests must encode the **desired** loop (e.g. after successful play, return to menu/search rather than exit—exact behavior: **read current `main.rs` and choose the smallest user-visible improvement** that matches HANDOFF item 14, then lock it in tests).  
- **GREEN / REFACTOR / merge / push / HANDOFF**.

#### D2 — HANDOFF **15** (`Box<dyn Selector>` or equivalent trait object)

- **Branch:** `cursor/d2-selector-dyn-trait-tdd`  
- **Worktree:** `"$WT_PARENT/d2-selector-dyn-trait"`  
- **Base:** `origin/main` after D1.  
- **TDD:** **RED** tests through the selector **factory** / trait boundary (existing selection behavior must not regress). **GREEN** refactor `src/ui/*` + `src/main.rs` call sites.  
- **Gate + merge + push + HANDOFF** `[x]` for **15**.

---

### Final checklist

- [ ] `main` at `origin/main` with **all** waves merged.  
- [ ] Last run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` **green**.  
- [ ] `HANDOFF.md` reflects items completed in **this** run (**11, 18, 19, 35, 36, 37, 23, 27, 28, 29, 14, 15**); **Last updated** set to today. Do not uncheck items already `[x]` from earlier sessions.  
- [ ] Worktrees removed; obsolete local branches pruned where safe.  
- [ ] Write a short **summary comment** (in your final message to the user) listing merge commits and any conflicts you resolved.

### If subagents are unavailable

Fall back to **sequential worktrees** yourself: same branches, same TDD order, same merges—do not skip tests.

## END PASTE ABOVE THIS LINE

---

### Notes for the human (not part of the agent paste)

- Cursor may still show permission UI depending on workspace settings; use **auto-run** or project settings that allow `git` / `cargo` / network without per-step approval.  
- If `REPO` or `WT_PARENT` differs on your machine, do a find-replace before pasting.  
- A1’s full body lives in **HANDOFF.md** appendix—keep that appendix in sync if you edit the slice spec.
