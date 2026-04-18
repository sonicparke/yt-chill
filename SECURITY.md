# Security notes

## `editor` configuration

The `editor` field in `config.json` is passed as the program name to `std::process::Command` when you use `--edit` / `-e`. That means it can run **any executable** the user can invoke, with the config file path as its argument—same power as typing the command in a shell.

**Practical risk:** Low for typical single-user CLI use: only someone who can already edit your config or environment can point `editor` at a malicious binary. Treat `config.json` like shell startup files: do not merge untrusted snippets into it, and avoid running `yt-chill --edit` with a hijacked `EDITOR` / `PATH` if you are in a hostile environment.

## Network

The app fetches YouTube pages over HTTPS using a fixed browser-like user agent. It does not send credentials to YouTube.
