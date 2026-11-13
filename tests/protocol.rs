#[path = "../src/transport.rs"]
mod transport;

use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn real_child_transport_handshakes_streams_and_reaps() {
    let executable = format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR"));
    let backend = transport::Backend::start(&executable, "/tmp", || {}).unwrap();
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
    assert!(
        !std::path::Path::new(&format!("/proc/{pid}")).exists(),
        "backend must be reaped on disconnect"
    );
}
