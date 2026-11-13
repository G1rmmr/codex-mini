# codex-mini

A small window for Codex.
Rust. egui. Linux, Windows, macOS.

Projects, chat history, streaming replies, approvals. Korean UI.
Uses your installed Codex CLI and its existing login.

## Release archives

Linux: run `./codex-mini`. Windows: open `codex-mini.exe`. macOS: open `Codex Mini.app`.
Install Codex CLI separately and run `codex login` first. If discovery fails, set its path in Mini's global settings.

Windows builds are unsigned. macOS bundles are ad-hoc signed, not notarized.

## Run from source

Rust 1.92+. OpenGL. X11 or XWayland. [Codex CLI](https://github.com/openai/codex).
Tested with CLI 0.160.1.

```sh
codex login
./scripts/build.sh
./run.sh --cwd /path/to/project
```

Pick a project. Click `연결` (Connect). Type.
`Ctrl+Enter` sends. `Enter` adds a line. `Esc` closes settings.

Korean text uses Nanum Gothic or Noto Sans CJK on Linux, Apple SD Gothic Neo on macOS, and Malgun Gothic on Windows.
A custom font works too: `CODEX_MINI_FONT=/path/to/font.ttf ./run.sh`.

## Spend less. Maybe.

Open `설정` → `사용량·절약` (Settings → Usage / Savings).

- Actual input, cached input, output and reasoning counts. Last 200 turns. JSON export to clipboard.
- Per-project reasoning effort. Start with `medium`. Raise it for difficult work.
- Exclude unused skills and MCP servers. New chats pick up the changes.
- Optional RTK filter. Plain `cargo test`, `clippy`, `build`, `check` commands only.

```sh
./scripts/install-rtk.sh
```

The automatic RTK hook supports Bash on Linux and macOS. It is unavailable on Windows.

Restart Mini. Tick `Cargo 로그 필터 (RTK)` in Usage / Savings. Start a new chat.
The installer pins RTK OSS 1.2.0 and checks its SHA-256.

RTK may omit useful detail. Follow its recovery hint where available, or disable the filter and rerun.
Its telemetry is disabled for Mini's rewritten commands.

Cached tokens are part of input. Reasoning tokens are part of output.
Resumed chats may show partial measurements. These are counts, not a savings claim.
The bill remains your problem.

## Local files

Settings and usage: `~/.config/codex-mini/` on Linux, `~/Library/Application Support/codex-mini/` on macOS, `%APPDATA%\codex-mini\` on Windows. `XDG_CONFIG_HOME` overrides this on all platforms.
Usage records contain counts and run metadata, not prompts or replies.
Codex still stores its own sessions. RTK may store command output for recovery.

Project savings options leave shared Codex configuration alone.
The separate global settings editor changes shared files and keeps backups.

## Checks

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
./run.sh --diagnose
```

The diagnostic checks the app-server connection. No model request.

## Licenses

Mini: [MIT](../LICENSE).

| Dependency | License |
| --- | --- |
| eframe / egui | MIT OR Apache-2.0 |
| serde / serde_json | MIT OR Apache-2.0 |
| libc | MIT OR Apache-2.0 |
| toml_edit | MIT OR Apache-2.0 |
| egui_kittest (tests) | MIT OR Apache-2.0 |
| RTK OSS (optional) | Apache-2.0 |

Dependency versions, transitive licenses and bundled-font notices:
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).
