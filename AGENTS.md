# AGENTS.md instructions for /Users/bradmcalister/Documents/DEV/yt-chill

Use sub-agents whenever parallelization is likely to improve speed, coverage, or quality.

Default behavior:
For all coding tasks, use the build-new-thing skill.

After new work has been done:
Always run `cargo install --path .` after verification so the local `yt-chill` binary is updated for testing.
