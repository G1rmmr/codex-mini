#[path = "../src/transport.rs"]
mod transport;

use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn real_child_transport_handshakes_streams_and_reaps() {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| {
        if cfg!(windows) {
            "python".into()
        } else {
            "python3".into()
        }
    });
    let mut command = std::process::Command::new(python);
    command
        .arg(format!(
            "{}/tests/mock_codex.py",
            env!("CARGO_MANIFEST_DIR")
        ))
        .env("PYTHONIOENCODING", "utf-8")
        .current_dir(std::env::temp_dir());
    let backend = transport::Backend::start_command(command, || {}).unwrap();
    let pid = backend.pid();
    backend
        .send(json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"codex_mini"}}}))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut text = String::new();
    let mut completed = false;
    while Instant::now() < deadline {
        match backend
            .incoming
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
        {
            transport::Event::Message(v) => {
                if v["id"] == 1 {
                    backend
                        .send(json!({"method":"initialized","params":{}}))
                        .unwrap();
                    backend.send(json!({"id":2,"method":"thread/start","params":{"sandbox":"workspace-write","approvalPolicy":"on-request"}})).unwrap();
                } else if v["id"] == 2 {
                    backend.send(json!({"id":3,"method":"turn/start","params":{"threadId":"mock-thread","input":[{"type":"text","text":"hello 한글"}]}})).unwrap();
                }
                if v["method"] == "item/agentMessage/delta" {
                    text.push_str(v["params"]["delta"].as_str().unwrap());
                }
                if v["method"] == "turn/completed" {
                    completed = true;
                    break;
                }
            }
            transport::Event::Closed(reason) => panic!("{reason}"),
            transport::Event::Diagnostic(line) => eprintln!("{line}"),
        }
    }
    assert!(completed);
    assert_eq!(text, "안녕하세요. Codex Mini 테스트 응답입니다.");
    drop(backend);
    #[cfg(unix)]
    assert_eq!(
        unsafe { libc::kill(pid as i32, 0) },
        -1,
        "backend must be reaped"
    );
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_OBJECT_0},
            System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        };
        let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !process.is_null() {
            assert_eq!(WaitForSingleObject(process, 1000), WAIT_OBJECT_0);
            CloseHandle(process);
        }
    }
}

#[test]
fn missing_backend_returns_an_error() {
    let missing = std::env::temp_dir().join("codex-mini-nonexistent-backend-6fe9c8");
    assert!(
        transport::Backend::start(
            &missing.to_string_lossy(),
            &std::env::temp_dir().to_string_lossy(),
            || {}
        )
        .is_err()
    );
}
