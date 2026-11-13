# Builds

Rust is pinned in `rust-toolchain.toml`. Install rustup first.

- Linux x86_64: `./scripts/build.sh`. Requires a C compiler, pkg-config, X11 development headers and OpenGL. Ubuntu: `sudo apt install build-essential pkg-config libx11-dev libxi-dev libxrandr-dev libxkbcommon-dev libgl1-mesa-dev`.
- macOS Apple Silicon: `xcode-select --install`, then `./scripts/build.sh`. The app bundle targets macOS 11+.
- Windows x86_64: install Visual Studio Build Tools with Desktop development with C++, then run `./scripts/build.ps1` in PowerShell. Use the native Windows Codex CLI.

The app uses an installed Codex CLI. It is not bundled. Run `codex login` first. On Windows, complete Codex's first-run sandbox setup in the terminal. If CLI discovery fails, set `CODEX_MINI_CODEX` or its path in Mini's global settings.

## CI

Pushes to `main` or `codex/**`, pull requests, and manual runs execute format, Clippy, tests, release builds and packaging on all three targets. Tests use a Python fixture. No API keys or model calls.

Download archives from the run's Artifacts section. Artifacts expire after 14 days. Each archive includes the app, usage notes and dependency license text. RTK is optional and downloaded separately.

## Release

Update `Cargo.toml` and `Cargo.lock`. Commit to `main`. Push a matching tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

A tag must match the package version and point to a commit on `main`. The workflow repeats the checks, builds all three archives, then publishes a GitHub Release with `SHA256SUMS`. Only the publishing job has write permission. Existing releases are not overwritten.

Windows builds statically link the MSVC runtime. No separate Visual C++ runtime installation is required. Windows builds are unsigned. macOS bundles are ad-hoc signed, not Developer ID signed or notarized. OS security prompts may appear. Distribution signing needs the maintainer's certificates; none are stored in this repository.

For local packaging, install `cargo-about` 0.9.3, then generate notices before packaging:

```sh
cargo about generate --locked --fail about.hbs -o dist/THIRD_PARTY_LICENSES.html
./scripts/package.sh
```

On Windows, use `./scripts/package.ps1` for the last step.
