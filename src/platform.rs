use std::{env, ffi::OsString, path::PathBuf};

fn nonempty(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|s| !s.is_empty())
}

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let home = nonempty("USERPROFILE").or_else(|| nonempty("HOME"));
    #[cfg(not(windows))]
    let home = nonempty("HOME");
    home.map(PathBuf::from)
}

pub fn settings_dir() -> Option<PathBuf> {
    if let Some(path) = nonempty("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(path).join("codex-mini"));
    }
    #[cfg(windows)]
    {
        nonempty("APPDATA")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|p| p.join("AppData/Roaming")))
            .map(|p| p.join("codex-mini"))
    }
    #[cfg(target_os = "macos")]
    {
        home_dir().map(|p| p.join("Library/Application Support/codex-mini"))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        home_dir().map(|p| p.join(".config/codex-mini"))
    }
}

/// Finder and Explorer may not inherit the shell's PATH.
pub fn codex_binary() -> String {
    let mut dirs: Vec<PathBuf> = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default();
    if let Some(home) = home_dir() {
        dirs.extend([home.join(".cargo/bin"), home.join(".local/bin")]);
    }
    #[cfg(target_os = "macos")]
    dirs.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    #[cfg(windows)]
    if let Some(appdata) = nonempty("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("npm"));
    }
    #[cfg(windows)]
    let names = ["codex.exe", "codex.cmd", "codex.bat"];
    #[cfg(not(windows))]
    let names = ["codex"];
    for dir in dirs {
        for name in names {
            let path = dir.join(name);
            if path.is_file() {
                return path.to_string_lossy().into_owned();
            }
        }
    }
    "codex".into()
}

#[cfg(windows)]
pub fn attach_console() {
    // Diagnostics use the launching terminal when present; hooks retain their pipes.
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}
