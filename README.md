A small window for Codex.

- Rust / Cargo
- egui / eframe — MIT OR Apache-2.0
- serde / serde_json — MIT OR Apache-2.0
- libc / windows-sys — MIT OR Apache-2.0
- toml_edit — MIT OR Apache-2.0
- egui_kittest — MIT OR Apache-2.0
- RTK OSS — Apache-2.0
- cargo-about — MIT OR Apache-2.0
- GitHub Actions

## Build

Rust. [Codex CLI](https://github.com/openai/codex). Linux needs OpenGL and X11/XWayland.

Linux / macOS:

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

Run `codex login` once. Pick a folder. Click `연결` (Connect).
`Ctrl+Enter` sends. `Enter` adds a line.

[Usage](docs/USAGE.md) · [Builds and releases](docs/BUILD.md) · [Licenses](THIRD_PARTY_NOTICES.md)
