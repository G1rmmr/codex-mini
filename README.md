A small window for Codex.

- Rust / Cargo
- Iced — MIT
- tiny-skia — BSD-3-Clause
- serde / serde_json — MIT OR Apache-2.0
- libc / windows-sys — MIT OR Apache-2.0
- toml_edit — MIT OR Apache-2.0
- tray-icon — MIT OR Apache-2.0
- zbus — MIT
- sysinfo — MIT
- tokio / open / png — MIT or permissive dependency licenses
- RTK OSS — Apache-2.0
- cargo-about — MIT OR Apache-2.0
- GitHub Actions

## Build

Rust. [Codex CLI](https://github.com/openai/codex). The default build uses software rendering with X11 or Wayland on Linux. No WebView or GPU is required.

Linux / macOS (Apple Silicon):

```sh
./scripts/build.sh
./run.sh --cwd /path/to/project
```

Windows, PowerShell:

```powershell
.\scripts\build.ps1
.\run.ps1 --cwd C:\path\to\project
```

## Use

Install Codex CLI. Mini connects on launch and offers browser sign-in when needed. Pick a project to switch its working folder; project instructions are optional.
Choose the model in the chat header and change effort from the composer's sliders button.
Open another project or a new chat while a task is running; each conversation
keeps its own backend connection while working. Idle connections in other projects
close automatically; returning reconnects and restores the chat and draft.
Use the Chat menu or recent chats to return.
Background conversations continue receiving replies, approval requests and notifications.
Toggle Auto-accept in the composer options to accept pending and future command, file change
and permission requests for this connection. Questions still need your answer. Empty or
incomplete previews stay manual: allow the request once or decline it in the app.
Auto-accept skips these requests. It starts off and resets on disconnect.
Use a recent chat's `···` menu to move or delete it. Context compacts automatically at 80% of the reported window.
`Enter` sends. `Shift+Enter` adds a line.
Closing the window keeps work running; reopen it from the system tray (macOS: menu bar).
Use the chat header's `···` → Quit to stop it. Follow-up questions also send a system notification,
even when another request is pending or Auto-accept is on.
Task notifications can be switched off in Settings.
Settings: English / Korean. Light / dark.

Folded command logs, Markdown replies, on-demand history, a task queue (one concurrent task by default), and on-demand process resource diagnostics.

[Usage](docs/USAGE.md) · [Builds and releases](docs/BUILD.md) · [Licenses](THIRD_PARTY_NOTICES.md)
