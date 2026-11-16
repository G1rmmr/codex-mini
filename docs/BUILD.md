# Builds

Rust is pinned in `rust-toolchain.toml`. Install rustup first. Release scripts use explicit target triples and copy the executable from `target/<triple>/release/` into `dist/`.

- Linux x86_64: `./scripts/build.sh`. Uses X11/Wayland and tiny-skia software rendering by default. Ubuntu build dependencies: `sudo apt install build-essential pkg-config libx11-dev libxi-dev libxrandr-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev`. GPU libraries are needed only by the optional GPU configuration and native windowing dependencies. No WebKitGTK or GTK is required.
- macOS Apple Silicon: `xcode-select --install`, then `./scripts/build.sh`. The app bundle targets macOS 11+.
- Windows x86_64: install Visual Studio Build Tools with Desktop development with C++, then run `./scripts/build.ps1` in PowerShell. Use the native Windows Codex CLI.

The app uses an installed Codex CLI. It is not bundled. Run `codex login` first. On Windows, complete Codex's first-run sandbox setup in the terminal. If CLI discovery fails, set `CODEX_MINI_CODEX` or its path in Mini's global settings.

## CI

Pushes to `main` or `codex/**`, pull requests, and manual runs execute format, Clippy, tests, release builds and packaging on all three targets. Tests use a Python fixture. No API keys or model calls. CI also checks the optional GPU feature and captures software-rendered UI PNGs on all three platforms. Linux uses Xvfb; UI text uses the embedded Pretendard font. Screenshot smoke tests use synthetic data and do not connect to Codex, save settings, or create a tray icon.

Download archives from the run's Artifacts section. Artifacts expire after 14 days. Each archive includes the app, usage notes and dependency license text. RTK is optional and downloaded separately.

## Release

Update `Cargo.toml` and `Cargo.lock`. Commit to `main`. Push a matching tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

A tag must match the package version and point to a commit on `main`. The workflow repeats the checks, builds all three archives, then publishes a GitHub Release with `SHA256SUMS`. Only the publishing job has write permission. Existing releases are not overwritten.

Windows builds statically link the MSVC runtime. No separate Visual C++ runtime installation is required. Windows builds are unsigned. macOS bundles are ad-hoc signed, not Developer ID signed or notarized. OS security prompts may appear. Distribution signing needs the maintainer's certificates; none are stored in this repository.

Release scripts remap compiler-generated source/home paths to generic build paths. The publication gate checks source, commit identities and release binaries without printing matched sensitive values:

```sh
python scripts/check-publication.py --history --binary ./dist/codex-mini
git config --local core.hooksPath .githooks
```

Use `codex-mini.exe` on Windows. Commit with a public handle and GitHub noreply email. CI enforces this for reachable commits; a local pre-push hook provides the same check. These checks do not remove old GitHub PR references/caches or copies other people already downloaded. Local history rollback bundles, benchmark reports/profiles and user settings must never be included in a public upload. Use the packaging scripts' selected payload, not a ZIP of the whole workspace.

For local packaging, install `cargo-about` 0.9.3, then generate notices before packaging:

```sh
cargo about generate --locked --fail about.hbs -o dist/THIRD_PARTY_LICENSES.html
./scripts/package.sh
```

On Windows, use `./scripts/package.ps1` for the last step.

## Renderer and UI verification

The default release builds tiny-skia only, without wgpu, a WebView, or a bundled browser. GPU rendering is opt-in:

```sh
./scripts/build.sh --features gpu
```

```powershell
./scripts/build.ps1 -Gpu
```

To render synthetic test content and exit, with no Codex process or model call:

```sh
./dist/codex-mini --ui-smoke dist/ui-dark.png
./dist/codex-mini --ui-smoke dist/ui-light-compact.png --light --compact
```

Use `codex-mini.exe` on Windows; on headless Linux prefix with `xvfb-run -a`. `--interactive` keeps the synthetic preview open for manual keyboard and layout checks. Real IME composition must also be tested with the target platform's input method (for example IBus/Fcitx on Linux); injecting a Korean string is not a complete IME test.

## Idle overhead benchmark

Install Python's `psutil`, then run against a release build:

```sh
python scripts/benchmark.py --mini ./dist/codex-mini --codex /path/to/codex --output dist/benchmark.json
```

On Windows use `./dist/codex-mini.exe`. Optionally pass `--desktop-pid PID` to observe an already-running desktop client. The script never restarts that client. It launches isolated Mini settings, connects a real Codex backend and reads account/model metadata without submitting a prompt. Each of three trials warms up for three seconds and samples for fifteen seconds. Only its own newly launched processes are terminated afterward.

The JSON includes raw samples, working set, Windows private committed memory, CPU (100% = one logical core) and warm startup through metadata readiness. CLI runs in `app-server` mode; terminal rendering is excluded. Mini uses an empty connected chat with normal tray/journal services. An existing desktop session is a contextual observation, not an equivalent fresh idle trial. Streaming speed, inference quality, GPU memory and low-end Linux behavior need separate measurements.

For performance comparison, run a release build with the same conversation and font on the actual low-spec Linux PC. Measure GUI and backend RSS/CPU separately in Resources, then close the panel before measuring idle cost externally. Windows measurements do not establish Linux performance.
