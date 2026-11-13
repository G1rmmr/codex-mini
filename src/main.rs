#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod efficiency;
mod global;
mod i18n;
mod platform;
mod preferences;
mod state;
mod transport;
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
    let mut settings = state::Settings::load();
    let mut binary = std::env::var("CODEX_MINI_CODEX").unwrap_or_else(|_| {
        if settings.codex_binary.is_empty() {
            platform::codex_binary()
        } else {
            settings.codex_binary.clone()
        }
    });
    let mut diagnose = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cwd" => settings.cwd = args.next().ok_or("--cwd requires a folder")?,
            "--codex" => binary = args.next().ok_or("--codex requires an executable")?,
            "--diagnose" => diagnose = true,
            "-h" | "--help" => {
                println!(
                    "Codex Mini — lightweight native Codex GUI\n\n  codex-mini [--cwd FOLDER] [--codex EXECUTABLE]\n  codex-mini --diagnose  (handshake + account/model check; no inference)\n\nUses your existing Codex CLI login. Ctrl+Enter sends; Enter inserts a newline."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    if diagnose {
        return diagnostic(&binary, &settings.cwd);
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Codex Mini")
            .with_app_id("codex-mini")
            .with_icon(eframe::egui::IconData {
                rgba: include_bytes!("../assets/codex-mini-64.rgba").to_vec(),
                width: 64,
                height: 64,
            })
            .with_inner_size([720.0, 520.0])
            .with_min_inner_size([560.0, 400.0]),
        renderer: eframe::Renderer::Glow,
        multisampling: 0,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Codex Mini",
        options,
        Box::new(move |cc| Ok(Box::new(app::MiniApp::new(cc, settings, binary)))),
    )?;
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
