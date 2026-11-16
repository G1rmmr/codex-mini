# codex-mini

A small window for Codex.
Rust. egui. Linux, Windows, macOS.

Projects, chat history, streaming replies, approvals. English / Korean UI.
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

Pick a project to connect automatically. You can optionally add instructions in Project Edit. `연결` (Connect) also starts or retries a connection. Select a model in the new chat composer, then type.
`Enter` sends. `Shift+Enter` adds a line. `Esc` closes settings.

Open Settings to choose English or 한국어 and Light or Dark. Changes apply immediately and persist across restarts. The default is Korean / Dark. Chats, project names and Codex replies keep their original text.

Korean text uses Nanum Gothic or Noto Sans CJK on Linux, Apple SD Gothic Neo on macOS, and Malgun Gothic on Windows.
A custom font works too: `CODEX_MINI_FONT=/path/to/font.ttf ./run.sh`.

## Chat controls

- Messages sent during a running task are added to that same task immediately. During context compaction or initial chat preparation, wait for the chat to be ready.
- Your messages appear on the right immediately and reconcile with server history. Failed follow-ups are restored to the composer, or kept in a copyable failure entry if you have already started a new draft.
- Switching projects connects the selected project automatically. Idle connections in other projects close; running tasks and pending approvals keep their connections until finished. Returning to a suspended chat reconnects and restores its history, model, effort and draft. Auto-accept resets when a connection closes.
- You can add projects during work. They are saved without switching away from the active task; select them whenever needed. Editing the active project waits until its task finishes, then saving reconnects automatically.
- Model selection lives in new chats, outside Settings. The account's model catalog supplies the choices.
- Change effort in the composer at any time. It applies to the next message; an in-flight response keeps its original effort. Model and effort are remembered per chat.
- `권장` (Recommended) marks a local, rule-based suggestion from project instructions and the available catalog. It does not call a model or guarantee the best choice. Simple/speed-focused instructions favor a lightweight catalog entry; otherwise the catalog default is suggested. Effort favors low, medium, or high within the model's supported values.
- Right-click a recent chat to move it to another project, delete it, copy its ID, or compact the active chat. The active chat's actions are also available in the Chat menu.
- Moving preserves history. On resume, future work uses the destination project's folder and instructions; existing files are not moved.
- Delete requires confirmation and permanently removes the Codex thread and its child threads. Working files and the separate usage journal remain. Failed requests keep the chat in the list.
- Automatic compaction runs after a successful turn when the reported latest context usage reaches 80% of the reported model context window. It waits for compaction completion before accepting another message. If the server omits the context-window measurement, Mini cannot trigger the threshold; Codex's own automatic compaction still follows its configuration. Manual compaction remains available.
- Project instructions are optional when creating, editing, sending or resuming a chat.
- Auto-accept handles pending and future command, file change and permission requests for the current connection. It starts off and resets on disconnect. Questions still need your answer. Requests with an empty or incomplete preview stay manual: allow them once or decline them in the app. Auto-accept skips them. Permission requests without permission details cannot be allowed.

## Background work and notifications

Closing the window keeps Codex and the current chat running. Use the tray menu's **Open** to return to the same conversation, including streamed replies and your draft. **App → Run in background** does the same without closing the window manually.

- Windows: the notification-area tray icon; left-click to reopen, right-click for the menu. Task notifications use the native tray notification API.
- macOS: the menu-bar status icon and its Open / Quit menu. Notifications use the system AppleScript notification service; allow notifications for the script notification sender in macOS settings if needed.
- Linux: a StatusNotifierItem in the desktop panel (for example LXQt or KDE), with native desktop notifications over the session D-Bus. The Linux build does not require GTK. If there is no compatible tray host, closing minimizes the window so it can still be restored from the taskbar.

**Settings → Task notifications** controls completion, failure and approval-required notifications. Notification contents include the project name and state. Completion is reported once per task; automatic context compaction does not send a completion alert. OS notification permissions and Do Not Disturb still apply.

To stop the app, choose **App → Quit app** or **Quit app** in the tray menu. Quitting during work asks for confirmation, then stops that app's Codex process. Hiding the window does not disconnect or restart Codex. Background work lasts for the current login session; this is not a boot-time service.

Native Linux tray and notification integration can be tested without affecting your desktop:

```sh
dbus-run-session -- env CODEX_MINI_PRIVATE_BUS_TEST=1 cargo test --locked native_tray_activation_and_notification -- --ignored --nocapture
```

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
# Optional rendered review (requires a GPU or software graphics adapter):
cargo test --locked --features ui-review render_chat_review -- --ignored --nocapture
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
