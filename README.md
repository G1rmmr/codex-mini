A small window for Codex.

- Rust / Cargo
- egui / eframe — MIT OR Apache-2.0
- serde / serde_json — MIT OR Apache-2.0
- libc / windows-sys — MIT OR Apache-2.0
- toml_edit — MIT OR Apache-2.0
- tray-icon — MIT OR Apache-2.0
- zbus — MIT
- egui_kittest — MIT OR Apache-2.0
- RTK OSS — Apache-2.0
- cargo-about — MIT OR Apache-2.0
- GitHub Actions

## Build

Rust. [Codex CLI](https://github.com/openai/codex). Linux needs OpenGL and X11/XWayland.

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

Run `codex login` once. Pick a project to connect automatically; project instructions are optional.
Choose the model in a new chat and change effort in the composer.
Open another project or a new chat while a task is running; each conversation
keeps its own backend connection while working. Idle connections in other projects
close automatically; returning reconnects and restores the chat and draft.
Use the Chat menu or recent chats to return.
Background conversations continue receiving replies, approval requests and notifications.
Toggle Auto-accept beside these controls to accept pending and future command, file change
and permission requests for this connection. Questions still need your answer. Empty or
incomplete previews stay manual: allow the request once or decline it in the app.
Auto-accept skips these requests. It starts off and resets on disconnect.
Right-click recent chats to move or delete them. Context compacts automatically at 80% of the reported window.
`Enter` sends. `Shift+Enter` adds a line.
Closing the window keeps work running; reopen it from the system tray (macOS: menu bar).
Use App → Quit app to stop it. Follow-up questions also send a system notification,
even when another request is pending or Auto-accept is on.
Task notifications can be switched off in Settings.
Settings: English / Korean. Light / dark.

[Usage](docs/USAGE.md) · [Builds and releases](docs/BUILD.md) · [Licenses](THIRD_PARTY_NOTICES.md)
