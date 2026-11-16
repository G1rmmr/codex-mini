#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod controller;
mod desktop;
mod efficiency;
mod global;
mod i18n;
mod icons;
mod platform;
mod preferences;
mod recommendation;
mod resources;
mod runtime;
mod state;
mod transport;
mod typography;
mod ui;
mod usage;

use serde_json::json;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    if std::env::args().any(|a| matches!(a.as_str(), "--diagnose" | "--help" | "-h")) {
        platform::attach_console();
    }
    if std::env::args().nth(1).as_deref() == Some("--rtk-hook") {
        let path = std::env::args().nth(2).ok_or("missing RTK path")?;
        return efficiency::run_hook(std::path::Path::new(&path));
    }
    if std::env::args().nth(1).as_deref() == Some("--ui-smoke") {
        let output = std::env::args()
            .nth(2)
            .ok_or("--ui-smoke requires an output PNG path")?;
        let mut settings = state::Settings::default();
        if std::env::args().any(|a| a == "--light") {
            settings.theme = preferences::Theme::Light;
        }
        return ui::run(settings, "unused".into(), Some(output.into()), None).map_err(Into::into);
    }
    let mut settings = state::Settings::load();
    let mut binary = std::env::var("CODEX_MINI_CODEX").unwrap_or_else(|_| {
        if settings.codex_binary.is_empty() {
            platform::codex_binary()
        } else {
            settings.codex_binary.clone()
        }
    });
    let mut diagnose = false;
    let mut benchmark = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cwd" => settings.cwd = args.next().ok_or("--cwd requires a folder")?,
            "--codex" => binary = args.next().ok_or("--codex requires an executable")?,
            "--diagnose" => diagnose = true,
            "--benchmark-idle" => {
                benchmark = Some(
                    args.next()
                        .ok_or("--benchmark-idle requires a report path")?
                        .into(),
                );
            }
            "-h" | "--help" => {
                println!(
                    "Codex Mini — lightweight native Codex GUI\n\n  codex-mini [--cwd FOLDER] [--codex EXECUTABLE]\n  codex-mini --diagnose  (handshake + account/model check; no inference)\n\nUses your existing Codex CLI login. Enter sends; Shift+Enter inserts a newline."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    if diagnose {
        return diagnostic(&binary, &settings.cwd);
    }
    if benchmark.is_some() && std::env::var_os("XDG_CONFIG_HOME").is_none() {
        return Err("--benchmark-idle requires an isolated XDG_CONFIG_HOME directory".into());
    }
    ui::run(settings, binary, None, benchmark)?;
    Ok(())
}

fn diagnostic(binary: &str, cwd: &str) -> Result<(), Box<dyn std::error::Error>> {
    let backend = transport::Backend::start(binary, cwd, || {})?;
    backend.send(json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"codex_mini","title":"Codex Mini","version":env!("CARGO_PKG_VERSION")}}}))?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut account = false;
    let mut models = false;
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match backend.incoming.recv_timeout(remaining)? {
            transport::Event::Message(v) => {
                if let Some(e) = v.get("error") {
                    return Err(e.to_string().into());
                }
                match v["id"].as_u64() {
                    Some(1) => {
                        println!("initialize: OK (backend PID {})", backend.pid());
                        backend.send(json!({"method":"initialized","params":{}}))?;
                        backend.send(
                            json!({"id":2,"method":"account/read","params":{"refreshToken":false}}),
                        )?;
                        backend
                            .send(json!({"id":3,"method":"model/list","params":{"limit":50}}))?;
                    }
                    Some(2) => {
                        println!(
                            "account: {}",
                            v["result"]["account"]["type"]
                                .as_str()
                                .unwrap_or("not signed in / custom provider")
                        );
                        account = true;
                    }
                    Some(3) => {
                        let count = v["result"]["data"].as_array().map_or(0, Vec::len);
                        println!("model/list: OK ({count} models)");
                        models = true;
                    }
                    _ => {}
                }
                if account && models {
                    println!("Protocol check complete; no inference request sent.");
                    return Ok(());
                }
            }
            transport::Event::Closed(reason) => return Err(reason.into()),
            transport::Event::Diagnostic(_) => {}
        }
    }
    Err("Codex handshake timed out".into())
}
