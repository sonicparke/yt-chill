# Autonomous orchestration prompt (TUI polish)

**Purpose:** Paste the entire block below into a **single long-running Cursor agent / Composer session** that can run terminals and subagents **without** asking you to approve each command. Use a profile with **auto-run / YOLO** (or approve-all) for `Shell` and `Task`, and grant **`all`** (or `git_write` + `full_network`) so `git`, `cargo`, and worktrees are not sandbox-blocked.

**Repo root (edit if yours differs):** `REPO=/Users/bradmcalister/DEV/yt-chill`  
**Worktrees parent:** `WT_PARENT=/Users/bradmcalister/DEV/yt-chill-wt`  
**Remote:** `origin`  
**Integration branch:** always merge into `main`, then `git push origin main`.

---

## BEGIN PASTE BELOW THIS LINE

You are the **parent orchestrator** for a **TUI polish pass** on the `yt-chill` Rust CLI. Your job is to improve the **menu flow, copy, prompt UX, selector polish, and playback/status messaging** using **git worktrees**, **subagents** (`Task` with `subagent_type: best-of-n-runner` where available), **focused validation**, **merge + conflict resolution**, **HANDOFF.md updates**, and **push to `origin/main`**. The user will **not** be available to approve commands. Use broad permissions and do not stop for optional confirmations.

### Global rules

1. **Before anything:** `cd "$REPO" && git fetch origin && git checkout main && git pull origin main`. Work from a clean tree.
2. **After every merge into `main`:**  
   `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`  
   If anything fails, **fix on `main`** (minimal fix), commit, push, then continue.
3. **Subagent contract:** Each slice runs in its **own worktree** and **own branch** `cursor/<slice>`. Subagents: **do not push** from worktrees; return **branch name + commit SHAs + files touched + manual verification notes + any new test names**.
4. **Scope:** Polish the TUI only. Favor **better copy, clearer prompts, smoother flow, more consistent selector behavior, and cleaner status output**. Do **not** broaden into unrelated backend/parser/storage work unless strictly required for a TUI behavior change.
5. **Testing posture:** Add/update tests where a small focused test materially locks behavior. Do **not** force brittle interactive end-to-end TUI tests if they add more noise than safety.
6. **Merging:** After a subagent returns, merge with `git merge --no-ff <branch> -m "Merge <branch>: <short summary>"`. Resolve conflicts manually; keep the most polished user-facing result, not whichever side happens to compile first.
7. **HANDOFF.md:** Add a `### Session YYYY-MM-DD` note describing the TUI polish run, update **Last updated**, and summarize the user-visible improvements landed in this run. Do **not** reopen or uncheck completed items.
8. **Cleanup:** Remove worktrees when done: `git worktree remove <path>` and `git branch -d <branch>` for merged topic branches if safe.

### Critical ownership rule

`src/main.rs` is the highest-conflict file in this repo. Only **one active slice** may own it at a time. If later slices need `src/main.rs`, they must branch from the **post-merge `main` SHA** of the previous `main.rs`-touching slice. Do **not** run multiple parallel branches that all edit `src/main.rs`.

### Recommended polish rubric

- Menus should make the next action obvious.
- Cancel / empty / no-results states should feel intentional, not abrupt.
- Prompt labels should be short, consistent, and not repetitive.
- Status output should feel calm and readable.
- `fzf` and dialoguer paths should feel like the same product, even if capabilities differ.

---

### Wave T1 — menu flow + copy polish (owns `src/main.rs`)

- **Branch:** `cursor/t1-menu-flow-copy-polish`  
- **Worktree:** `"$WT_PARENT/t1-menu-flow-copy-polish"`  
- **Base:** `git worktree add -b cursor/t1-menu-flow-copy-polish "$WT_PARENT/t1-menu-flow-copy-polish" origin/main`
- **Files:** `src/main.rs` (+ `README.md` only if behavior/documentation changes are worth noting)
- **Spec:**  
  - Improve the wording and consistency of main menu labels, action prompts, empty states, and success/error copy where it helps the experience.
  - Make post-action flow feel deliberate: menu returns, cancellations, and one-shot commands should read cleanly and avoid awkward dead ends where practical.
  - Keep behavior changes **small and user-facing**; do not re-architect beyond what is needed for polish.
  - Add focused tests only if you extract a small pure helper to lock a new flow/copy rule.
- **Gate:** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`

**Merge T1:** checkout `main` → merge **T1** → gates → push.

---

### Wave T2 — dialoguer UX polish (after T1 is merged)

- **Branch:** `cursor/t2-dialoguer-ux-polish`  
- **Worktree:** `"$WT_PARENT/t2-dialoguer-ux-polish"`  
- **Base:** `origin/main` **after T1 fully merged and pushed**
- **Files:** `src/ui/dialoguer_selector.rs` (`src/main.rs` only if absolutely necessary and only from the post-T1 SHA)
- **Spec:**  
  - Improve dialoguer-based selection UX: prompt clarity, default behavior, cancel handling, and any theme/setup polish that increases consistency with the rest of the app.
  - Prefer local improvements in `dialoguer_selector.rs`; only touch `main.rs` if an integration seam truly needs adjustment.
  - Add narrow tests if a pure adapter/helper is introduced; otherwise validate manually and keep the branch small.
- **Gate:** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`

**Merge T2:** checkout `main` → merge **T2** → gates → push.

---

### Wave T3 — fzf UX polish (parallel with T2 only if it avoids `main.rs`)

- **Branch:** `cursor/t3-fzf-ux-polish`  
- **Worktree:** `"$WT_PARENT/t3-fzf-ux-polish"`  
- **Base:** `origin/main` **after T1 fully merged and pushed**
- **Files:** `src/ui/fzf.rs`
- **Spec:**  
  - Polish `fzf` prompt/flags/layout so it feels consistent with the app’s other prompts.
  - Favor improvements such as clearer prompt text, stable layout defaults, and output handling that reduces visual rough edges.
  - Do **not** add shell-dependent tests that are likely to be flaky in CI. Keep this slice simple and robust.
- **Gate:** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`

**Merge T3:** checkout `main` → merge **T3** → gates → push.

---

### Optional Wave T4 — playback/status messaging polish

Run this **only if** T1–T3 reveal obvious remaining rough edges in playback/status output.

- **Branch:** `cursor/t4-playback-status-polish`  
- **Worktree:** `"$WT_PARENT/t4-playback-status-polish"`  
- **Base:** `origin/main` after T2/T3 are merged
- **Files:** `src/core/player.rs`, `src/main.rs` (only if necessary)
- **Spec:**  
  - Refine buffering / playing / completion / failure messages so they match the polished TUI tone.
  - Preserve current functionality and avoid reworking playback internals.
- **Gate:** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`

**Merge T4:** checkout `main` → merge **T4** → gates → push.

---

### Finalization

1. Update `HANDOFF.md` with a new `### Session YYYY-MM-DD — TUI polish` section that records:
   - which slices ran,
   - what user-facing TUI changes landed,
   - any notable manual verification performed,
   - whether T4 was skipped.
2. Commit the HANDOFF update on `main` and push.

### Final checklist

- [ ] `main` at `origin/main` with all intended TUI slices merged.  
- [ ] Last run: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` **green**.  
- [ ] `HANDOFF.md` has a new session note for this TUI polish run.  
- [ ] Worktrees removed; obsolete local branches pruned where safe.  
- [ ] Final user summary lists merged branches, any conflicts resolved, and whether playback/status messaging needed a separate T4 pass.

### If subagents are unavailable

Fall back to **sequential worktrees** yourself: same branches, same ownership rule for `src/main.rs`, same gates, same HANDOFF update.

## END PASTE ABOVE THIS LINE

---

### Notes for the human (not part of the agent paste)

- T1 intentionally owns `src/main.rs` first to reduce merge churn.
- T2 and T3 can run in parallel **only if** T2 stays out of `src/main.rs`; otherwise run T2 then T3.
- This prompt is intentionally lighter than a feature/refactor plan: it optimizes for user-facing finish, not architecture change.
