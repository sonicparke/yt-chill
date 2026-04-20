# Autonomous orchestration prompt (test expansion)

**Purpose:** Paste the entire block below into a **single long-running Cursor agent / Composer session** that can run terminals and subagents **without** asking you to approve each command. Use a profile with **auto-run / YOLO** (or approve-all) for `Shell` and `Task`, and grant **`all`** (or `git_write` + `full_network`) so `git`, `cargo`, and worktrees are not sandbox-blocked.

**Repo root (edit if yours differs):** `REPO=/Users/bradmcalister/DEV/yt-chill`  
**Worktrees parent:** `WT_PARENT=/Users/bradmcalister/DEV/yt-chill-wt`  
**Remote:** `origin`  
**Integration branch:** always merge into `main`, then `git push origin main`.

---

## BEGIN PASTE BELOW THIS LINE

You are the **parent orchestrator** for a **test expansion pass** on the `yt-chill` Rust CLI. Your job is to increase confidence with **high-value automated tests** using **git worktrees**, **subagents** (`Task` with `subagent_type: best-of-n-runner` where available), **TDD where implementation changes are needed**, **merge + conflict resolution**, **HANDOFF.md updates**, and **push to `origin/main`**. The user will **not** be available to approve commands. Use broad permissions and do not stop for optional confirmations.

### Global rules

1. **Before anything:** `cd "$REPO" && git fetch origin && git checkout main && git pull origin main`. Work from a clean tree.
2. **After every merge into `main`:**  
   `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`  
   If anything fails, **fix on `main`** (minimal fix), commit, push, then continue.
3. **Subagent contract:** Each slice runs in its **own worktree** and **own branch** `cursor/<slice>`. Subagents: **do not push** from worktrees; return **branch name + commit SHAs + files touched + new test names + what behavior is now locked down**.
4. **Testing philosophy:** Prefer **unit tests, characterization tests, and fixture-driven parser tests**. Avoid brittle interactive end-to-end TUI automation unless a slice can clearly justify that it is stable and worth the cost.
5. **Implementation restraint:** This run is about **tests first**. If a testable seam does not exist, you may extract a **small pure helper** or a narrow adapter to enable testing, but do not turn the run into a broad refactor.
6. **TDD rule:** If a slice must change production code to support a test, use **RED → GREEN → optional REFACTOR** and report the failing test names that went red first.
7. **Merging:** After a subagent returns, merge with `git merge --no-ff <branch> -m "Merge <branch>: <short summary>"`. Resolve conflicts manually; keep the stronger tests unless they are genuinely redundant or flaky.
8. **HANDOFF:** Add a `### Session YYYY-MM-DD` note in **`agent-docs/HANDOFF.md`** describing the test expansion run, update **Last updated**, and summarize the new test coverage added in this run. Do **not** reopen or uncheck completed items.
9. **Cleanup:** Remove worktrees when done: `git worktree remove <path>` and `git branch -d <branch>` for merged topic branches if safe.

### TDD snippet (embed in any slice that changes Rust code)

```text
TDD (mandatory if production code changes): (1) RED — add failing tests; cargo test must show new failures.
(2) GREEN — minimal implementation until all tests pass.
(3) REFACTOR — optional; tests stay green.
(4) cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test.
Prefer two commits: test: … then refactor/feat: …
Do not push from the worktree. Report branch name, SHAs, new test names, files touched.
```

### Coverage priorities

1. **Pure helpers / config compatibility** — cheap, durable tests.
2. **YouTube parsing + tracing regressions** — fixture-driven, high value.
3. **State/flow behavior** — only by extracting small pure helpers if justified.
4. **Interactive UI behavior** — lowest priority unless it can be tested without flake.

### Target code areas (orientation)

- `src/core/youtube.rs` — parser / tracing regressions (S2)  
- `src/storage/cache.rs`, `src/storage/config.rs` — cache keys, load/migration edges (S1)  
- `src/utils/process.rs`, `src/utils/playback.rs` — PATH helpers, playback policy helpers (S1)  
- `src/main.rs` — **only** if extracting a tiny pure helper is justified (prefer a dedicated `src/utils/*.rs` module instead)  
- New small modules under `src/utils/` (e.g. flow/state helpers) are acceptable when they keep tests deterministic

---

### Wave S1 — pure helpers + config compatibility

- **Branch:** `cursor/s1-helper-config-tests`  
- **Worktree:** `"$WT_PARENT/s1-helper-config-tests"`  
- **Base:** `git worktree add -b cursor/s1-helper-config-tests "$WT_PARENT/s1-helper-config-tests" origin/main`
- **Files:** `src/storage/cache.rs`, `src/storage/config.rs`, `src/utils/process.rs`, `src/utils/playback.rs`
- **Spec:**  
  - Expand durable unit coverage for pure helpers and compatibility edges already present in the tree.
  - Prioritize cases such as unusual inputs, empty/pathological values, legacy config normalization, and helper invariants.
  - Keep this slice **test-heavy and production-light**; avoid changing behavior unless a tiny extraction/fix is required to make the tests meaningful.
- **Gate:** `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`

**Merge S1:** checkout `main` → merge **S1** → gates → push.

---

### Merge order (isolated slices)

**Sequential:** **S1 → S2 → S3 (optional)**. S1 and S2 both touch high-churn areas; do **not** start S2 until S1 is merged to `main` and pushed unless you are certain there is **zero** file overlap and both branches were created from the **same** pre-S1 `origin/main` SHA (default policy: still **sequential** for fewer surprises).

---

### Wave S2 — YouTube parser + tracing regression tests

- **Branch:** `cursor/s2-youtube-parser-tests`  
- **Worktree:** `"$WT_PARENT/s2-youtube-parser-tests"`  
- **Base:** `origin/main` **after S1 fully merged and pushed**
- **Files:** `src/core/youtube.rs` (+ fixture files if you choose to add them; update `Cargo.toml` only if strictly needed)
- **Spec:**  
  - Expand parser regression coverage around `extract_yt_initial_data`, search/channel parsing, and any high-signal fallback branches that are still lightly covered.
  - Prefer **fixture-driven** tests or concise inline JSON fixtures over broad integration tests.
  - Add/expand tracing assertions only where they stay stable and useful.
  - If parser code must be lightly reorganized for testability, use the TDD snippet and keep behavior unchanged.
- **Gate:** `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`

**Merge S2:** checkout `main` → merge **S2** → gates → push.

---

### Wave S3 — state/flow tests through pure helpers (sequential, only if justified)

Run this slice **only if** the current tree still lacks confidence around menu/playback flow and a **small pure extraction** can test it cleanly.

- **Branch:** `cursor/s3-state-flow-tests`  
- **Worktree:** `"$WT_PARENT/s3-state-flow-tests"`  
- **Base:** `origin/main` after S1 and S2 are merged
- **Files:** `src/utils/flow.rs` (extend if present), or a **new** narrow `src/utils/<name>.rs` module, with thin call-site wiring in `src/main.rs` **only** if unavoidable
- **Spec:**  
  - Identify the smallest state/flow rule worth locking down, for example a transition helper or policy function.
  - Prefer extending or adding a **small pure module** under `src/utils/` over growing `main.rs` test harness code.
  - Extract a **small pure helper** only if that is the cleanest way to test the behavior.
  - Do **not** attempt full interactive dialoguer/fzf e2e automation unless you can keep it deterministic and low-maintenance.
  - Use the TDD snippet if any production code changes.
- **Gate:** `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`

**Merge S3:** checkout `main` → merge **S3** → gates → push.

---

### Finalization

1. Update `agent-docs/HANDOFF.md` with a new `### Session YYYY-MM-DD — test expansion` section that records:
   - which slices ran,
   - how many tests were added,
   - the key behaviors now covered,
   - whether S3 was skipped.
2. Commit the HANDOFF update on `main` and push.

### Final checklist

- [ ] `main` at `origin/main` with all intended test slices merged.  
- [ ] Last run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` **green**.  
- [ ] `agent-docs/HANDOFF.md` has a new session note for this test expansion run.  
- [ ] Worktrees removed; obsolete local branches pruned where safe.  
- [ ] Final user summary lists merged branches, any conflicts resolved, total tests added, and whether S3 was skipped.

### If subagents are unavailable

Fall back to **sequential worktrees** yourself: same branches, same prioritization, same gates, same HANDOFF update.

## END PASTE ABOVE THIS LINE

---

### Notes for the human (not part of the agent paste)

- S1 and S2 are intentionally split so helper/config tests do not get tangled with parser fixtures.
- S3 is optional; prefer `src/utils/flow.rs` (or similar) over bloating `main.rs`—only do it if a tiny pure extraction makes it worth the cost.
- This prompt is biased toward reliable tests that will keep paying for themselves as the CLI evolves.
