use super::*;
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};

const INCOMPLETE_WARNING: &str = "미리보기가 비어 있거나 일부 내용이 생략되었습니다. 이 요청은 자동수락하지 않습니다. 직접 허용하거나 거절하세요.";
const MISSING_PERMISSIONS: &str = "요청한 권한 정보가 없어 허용할 수 없습니다.";
const COMMAND_APPROVAL: &str = "item/commandExecution/requestApproval";
const FILE_APPROVAL: &str = "item/fileChange/requestApproval";
const PERMISSION_APPROVAL: &str = "item/permissions/requestApproval";

fn app_fixture(language: Language) -> MiniApp {
    let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
    let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
    app.settings.language = language;
    app.settings.notifications = false;
    // An incomplete request must remain manual even with this setting enabled.
    app.auto_accept = true;
    app
}

fn connected_fixture(language: Language) -> MiniApp {
    let mut app = app_fixture(language);
    app.backend = Some(
        Backend::start(
            &format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR")),
            crate::transport::test_cwd(),
            || {},
        )
        .unwrap(),
    );
    app.ready = true;
    app
}

fn request(app: &mut MiniApp, id: &str, method: &str, params: Value) {
    app.server_request(json!({"id":id,"method":method,"params":params}));
}

fn approval_harness(app: MiniApp, zoom: f32) -> Harness<'static, MiniApp> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(560.0, 400.0))
        // Do not run main_ui: it consumes the fixture's echoed protocol response.
        .build_ui_state(|ui, app: &mut MiniApp| app.approval_ui(ui.ctx()), app);
    h.ctx.set_zoom_factor(zoom);
    h.run();
    h
}

fn assert_waiting_manually(h: &Harness<'_, MiniApp>) {
    assert_eq!(h.state().approvals.len(), 1);
    assert!(automatic_approval(&h.state().approvals[0]).is_none());
    assert!(
        matches!(
            h.state().backend.as_ref().unwrap().incoming.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ),
        "a manual request must not have sent an automatic response"
    );
}

fn button_rect(h: &Harness<'_, MiniApp>, label: &str) -> egui::Rect {
    // egui_kittest 0.35 exposes AccessKit bounds in physical pixels, while
    // pointer input and content_rect use logical points. Node::click does not
    // perform this conversion, so use real pointer events at the logical center.
    let rect = h.get_by_label(label).rect();
    egui::Rect::from_min_max(
        rect.min / h.ctx.pixels_per_point(),
        rect.max / h.ctx.pixels_per_point(),
    )
}

fn click_button(h: &Harness<'_, MiniApp>, label: &str) {
    let pos = button_rect(h, label).center();
    // Harness advances a frame for each queued event, matching Node::click.
    h.hover_at(pos);
    h.drag_at(pos);
    h.drop_at(pos);
}

fn assert_button_visible(h: &Harness<'_, MiniApp>, label: &str) {
    let rect = button_rect(h, label);
    assert!(
        h.ctx.content_rect().contains_rect(rect),
        "{label:?} must fit the minimum-size window: {rect:?} in {:?}",
        h.ctx.content_rect()
    );
}

fn assert_reply(h: &Harness<'_, MiniApp>, id: &str, expected: Value) {
    assert!(
        h.state().approvals.is_empty(),
        "request {id:?} was not answered ({:?}, zoom {}, error {:?})",
        h.state().settings.language,
        h.ctx.zoom_factor(),
        h.state().error
    );
    let event = h
        .state()
        .backend
        .as_ref()
        .unwrap()
        .incoming
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    let Event::Message(message) = event else {
        panic!("expected an approval response, received {event:?}");
    };
    assert_eq!(message["method"], "test/approval");
    assert_eq!(message["params"]["id"], id);
    assert_eq!(message["params"]["result"], expected);
}

#[test]
fn incomplete_commands_can_be_manually_accepted_with_pointer_or_keyboard() {
    // Cover both input methods, languages and zoom levels without a full matrix.
    for (language, zoom, oversized, key) in [
        (Language::Korean, 1.0, false, None),
        (Language::English, 1.5, false, Some(egui::Key::Space)),
        (Language::Korean, 1.5, true, Some(egui::Key::Enter)),
        (Language::English, 1.0, true, None),
    ] {
        let mut app = connected_fixture(language);
        let params = if oversized {
            json!({"command":"x".repeat(state::MAX_BODY + 1),"availableDecisions":["accept","decline"]})
        } else {
            json!({"availableDecisions":["accept","decline"]})
        };
        request(
            &mut app,
            "auto-test-manual-command",
            COMMAND_APPROVAL,
            params,
        );
        let mut h = approval_harness(app, zoom);
        assert!(!h.state().approvals[0].preview_complete);
        assert_waiting_manually(&h);
        h.get_by_label(language.text(INCOMPLETE_WARNING));
        let allow = language.text("이번 한 번 허용");
        assert!(!h.get_by_label(allow).accesskit_node().is_disabled());
        assert_button_visible(&h, allow);
        assert_button_visible(&h, language.text("거절"));
        if let Some(key) = key {
            h.get_by_label(allow).focus();
            h.run();
            h.key_press(key);
        } else {
            click_button(&h, allow);
        }
        h.run();
        assert_reply(&h, "auto-test-manual-command", json!({"decision":"accept"}));
    }
}

#[test]
fn omitted_file_preview_stays_manual_and_can_be_accepted_after_language_change() {
    let mut app = connected_fixture(Language::Korean);
    app.transcript.put(
        "file-item",
        "파일 변경",
        "--- src/main.rs\n+++ src/main.rs\n… [화면 표시 생략]",
        true,
        false,
    );
    request(
        &mut app,
        "auto-test-manual-file",
        FILE_APPROVAL,
        json!({"itemId":"file-item","reason":"한글 파일 변경"}),
    );
    let mut h = approval_harness(app, 1.5);
    assert!(!h.state().approvals[0].preview_complete);
    assert_waiting_manually(&h);
    h.get_by_label(Language::Korean.text(INCOMPLETE_WARNING));
    h.state_mut().settings.language = Language::English;
    h.run();
    h.get_by_label(Language::English.text(INCOMPLETE_WARNING));
    assert!(!h.state().approvals[0].preview_complete);
    assert!(h.state().approvals[0].preview.contains("한글 파일 변경"));
    assert_button_visible(&h, "Allow once");
    click_button(&h, "Allow once");
    h.run();
    assert_reply(&h, "auto-test-manual-file", json!({"decision":"accept"}));
}

#[test]
fn truncated_permissions_can_be_manually_granted_with_the_original_payload() {
    let mut app = connected_fixture(Language::English);
    let permissions = json!({
        "fileSystem":{"write":[format!("/tmp/{}", "경로".repeat(state::MAX_BODY / 6 + 1))]},
        "network":{"enabled":true}
    });
    request(
        &mut app,
        "auto-test-manual-permissions",
        PERMISSION_APPROVAL,
        json!({"permissions":permissions}),
    );
    let mut h = approval_harness(app, 1.0);
    assert!(!h.state().approvals[0].preview_complete);
    assert_waiting_manually(&h);
    h.get_by_label(Language::English.text(INCOMPLETE_WARNING));
    let allow = "Allow permissions · this turn";
    assert!(!h.get_by_label(allow).accesskit_node().is_disabled());
    assert_button_visible(&h, allow);
    click_button(&h, allow);
    h.run();
    assert_reply(
        &h,
        "auto-test-manual-permissions",
        json!({"permissions":permissions,"scope":"turn"}),
    );
}

#[test]
fn missing_or_nonobject_permissions_cannot_be_granted() {
    for (language, params) in [
        (Language::Korean, json!({})),
        (Language::English, json!({"permissions":null})),
        (Language::Korean, json!({"permissions":[]})),
        (Language::English, json!({"permissions":"missing details"})),
    ] {
        let mut app = connected_fixture(language);
        request(
            &mut app,
            "auto-test-invalid-permissions",
            PERMISSION_APPROVAL,
            params,
        );
        let mut h = approval_harness(app, 1.0);
        assert_waiting_manually(&h);
        h.get_by_label(language.text(MISSING_PERMISSIONS));
        assert!(
            h.get_by_label(language.text("요청한 권한 허용 · 이번 작업만"))
                .accesskit_node()
                .is_disabled()
        );
        click_button(&h, language.text("거절"));
        h.run();
        assert_reply(
            &h,
            "auto-test-invalid-permissions",
            json!({"permissions":{},"scope":"turn"}),
        );
    }
}

#[test]
fn incomplete_requests_still_respect_available_decisions() {
    for method in [COMMAND_APPROVAL, FILE_APPROVAL] {
        let mut app = connected_fixture(Language::Korean);
        request(
            &mut app,
            "auto-test-restricted-decision",
            method,
            json!({"availableDecisions":["decline"]}),
        );
        let mut h = approval_harness(app, 1.0);
        assert_waiting_manually(&h);
        h.get_by_label(INCOMPLETE_WARNING);
        assert!(
            h.get_by_label("이번 한 번 허용")
                .accesskit_node()
                .is_disabled()
        );
        assert!(!h.get_by_label("거절").accesskit_node().is_disabled());
        assert_button_visible(&h, "거절");
        click_button(&h, "거절");
        h.run();
        assert_reply(
            &h,
            "auto-test-restricted-decision",
            json!({"decision":"decline"}),
        );
    }

    let mut app = connected_fixture(Language::English);
    request(
        &mut app,
        "auto-test-accept-only",
        COMMAND_APPROVAL,
        json!({"availableDecisions":["accept"]}),
    );
    let mut h = approval_harness(app, 1.5);
    assert_waiting_manually(&h);
    h.get_by_label(Language::English.text(INCOMPLETE_WARNING));
    assert!(h.get_by_label("Decline").accesskit_node().is_disabled());
    assert_button_visible(&h, "Allow once");
    click_button(&h, "Allow once");
    h.run();
    assert_reply(&h, "auto-test-accept-only", json!({"decision":"accept"}));
}

#[test]
fn incomplete_command_and_valid_permission_requests_can_still_be_declined() {
    for (method, params, expected) in [
        (COMMAND_APPROVAL, json!({}), json!({"decision":"decline"})),
        (
            PERMISSION_APPROVAL,
            json!({"reason":"x".repeat(state::MAX_BODY + 1),"permissions":{"network":{"enabled":true}}}),
            json!({"permissions":{},"scope":"turn"}),
        ),
    ] {
        let mut app = connected_fixture(Language::English);
        request(&mut app, "auto-test-manual-decline", method, params);
        let mut h = approval_harness(app, 1.0);
        assert_waiting_manually(&h);
        h.get_by_label(Language::English.text(INCOMPLETE_WARNING));
        click_button(&h, "Decline");
        h.run();
        assert_reply(&h, "auto-test-manual-decline", expected);
    }
}

#[test]
fn user_questions_do_not_show_the_incomplete_approval_warning() {
    for language in [Language::Korean, Language::English] {
        let mut app = connected_fixture(language);
        request(
            &mut app,
            "auto-test-manual-question",
            "item/tool/requestUserInput",
            json!({"questions":[{"id":"direction","question":"Choose a direction"}]}),
        );
        let mut h = approval_harness(app, 1.5);
        assert_waiting_manually(&h);
        h.get_by_label("Choose a direction");
        assert!(
            h.query_by_label(language.text(INCOMPLETE_WARNING))
                .is_none()
        );
        assert!(
            h.query_by_label(language.text(MISSING_PERMISSIONS))
                .is_none()
        );
        let send = language.text("답변 보내기");
        assert!(h.get_by_label(send).accesskit_node().is_disabled());
        h.state_mut().approvals[0]
            .answers
            .insert("direction".into(), "Continue".into());
        h.run();
        assert!(!h.get_by_label(send).accesskit_node().is_disabled());
        assert_button_visible(&h, send);
        click_button(&h, send);
        h.run();
        assert_reply(
            &h,
            "auto-test-manual-question",
            json!({"answers":{"direction":{"answers":["Continue"]}}}),
        );
    }
}

#[cfg(feature = "ui-review")]
#[test]
#[ignore = "render manual approvals at minimum window size and 150% zoom"]
fn render_manual_approval_review() {
    for theme in [
        crate::preferences::Theme::Dark,
        crate::preferences::Theme::Light,
    ] {
        let mut app = app_fixture(Language::Korean);
        app.transcript.put(
            "review-file",
            "파일 변경",
            "--- src/main.rs\n+++ src/main.rs\n- println!(\"이전 동작\");\n+ println!(\"변경한 동작\");\n… [화면 표시 생략]",
            true,
            false,
        );
        request(
            &mut app,
            "review-file-approval",
            FILE_APPROVAL,
            json!({"itemId":"review-file","reason":"선택한 프로젝트의 파일을 수정합니다."}),
        );
        let mut h = approval_harness(app, 1.5);
        crate::preferences::configure(&h.ctx, theme);
        super::install_korean_font(&h.ctx);
        h.run();
        h.render()
            .unwrap()
            .save(format!("/tmp/codex-mini-manual-approval-{theme:?}-1.5.png"))
            .unwrap();
    }
}
