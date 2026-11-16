use super::*;
use egui_kittest::{Harness, kittest::Queryable};

fn fixture() -> MiniApp {
    let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
    let settings = Settings {
        cwd: "/tmp".into(),
        ..Default::default()
    };
    let mut app = MiniApp::new(
        &cc,
        settings,
        format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR")),
    );
    app.settings.projects[0].name = "Project A".into();
    app.settings
        .add_project("Project B", "/tmp", "B instructions")
        .unwrap();
    app.show_projects = true;
    app
}

fn pump(app: &mut MiniApp, done: impl Fn(&MiniApp) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done(app) {
        assert!(
            Instant::now() < deadline,
            "timed out: {} {:?}",
            app.status,
            app.error
        );
        app.tick_logic(&app.ui_context.clone());
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn connected() -> MiniApp {
    let mut app = fixture();
    app.connect(&app.ui_context.clone());
    pump(&mut app, |a| a.ready && a.pending.is_empty());
    app
}

fn click(h: &mut Harness<'_, MiniApp>, label: &str) {
    let point = h.get_by_label(label).rect().center() / h.ctx.pixels_per_point();
    h.hover_at(point);
    h.run();
    h.drag_at(point);
    h.step();
    h.drop_at(point);
    h.run();
}

#[cfg(unix)]
fn assert_reaped(pid: u32) {
    assert_eq!(
        unsafe { libc::kill(pid as i32, 0) },
        -1,
        "process {pid} must exit"
    );
}

#[test]
fn project_buttons_connect_and_release_idle_processes_with_drafts_at_zoom() {
    for theme in [
        crate::preferences::Theme::Light,
        crate::preferences::Theme::Dark,
    ] {
        for (zoom, size) in [
            (1.0, egui::vec2(720.0, 520.0)),
            (1.5, egui::vec2(560.0, 400.0)),
        ] {
            let mut app = fixture();
            app.settings.theme = theme;
            let a = app.settings.projects[0].id;
            let b = app.settings.projects[1].id;
            let mut h = Harness::builder()
                .with_size(size)
                .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
            crate::preferences::configure(&h.ctx, theme);
            h.ctx.set_zoom_factor(zoom);
            h.run();
            click(&mut h, "Project B");
            assert_eq!(h.state().settings.active_project, Some(b));
            assert!(h.state().backend.is_some());
            pump(h.state_mut(), |a| a.ready && a.pending.is_empty());
            h.state_mut().prompt = "B 작성 중".into();
            let b_pid = h.state().backend.as_ref().unwrap().pid();
            h.run();
            click(&mut h, "Project A");
            assert_eq!(h.state().settings.active_project, Some(a));
            assert!(h.state().backend.is_some());
            assert!(h.state().prompt.is_empty());
            #[cfg(unix)]
            assert_reaped(b_pid);
            pump(h.state_mut(), |a| a.ready && a.pending.is_empty());
            h.state_mut().prompt = "A 작성 중".into();
            h.run();
            click(&mut h, "Project B");
            assert_eq!(h.state().prompt, "B 작성 중");
            assert_ne!(h.state().backend.as_ref().unwrap().pid(), b_pid);
            let pid = h.state().backend.as_ref().unwrap().pid();
            h.state_mut().switch_project(b);
            assert_eq!(h.state().backend.as_ref().unwrap().pid(), pid);
            if h.ctx.content_rect().width() < 620.0 {
                h.key_press(egui::Key::Escape);
                h.run();
                assert!(!h.state().show_projects);
                h.get_by_label("보내기");
            }
        }
    }
}

#[test]
fn idle_chat_reconnects_and_resumes_after_project_round_trip() {
    let mut app = connected();
    let a = app.settings.active_project.unwrap();
    let b = app.settings.projects[1].id;
    app.prompt = "hello".into();
    app.submit();
    pump(&mut app, |a| {
        a.thread_id.is_some() && !a.busy && !a.loading && a.pending.is_empty()
    });
    let thread = app.thread_id.clone().unwrap();
    let pid = app.backend.as_ref().unwrap().pid();
    app.prompt = "돌아와서 보낼 초안".into();
    app.settings.effort = "low".into();
    app.auto_accept = true;
    app.switch_project(b);
    assert!(app.sessions[0].backend.is_none());
    assert!(!app.sessions[0].auto_accept);
    assert_eq!(app.sessions[0].thread_id.as_deref(), Some(thread.as_str()));
    assert!(!app.sessions[0].transcript.messages.is_empty());
    #[cfg(unix)]
    assert_reaped(pid);
    app.switch_project(a);
    assert!(app.backend.is_some() && !app.ready);
    assert_eq!(app.resume_after_connect.as_ref().unwrap().id, thread);
    pump(&mut app, |a| {
        a.thread_id.as_deref() == Some(thread.as_str()) && !a.loading
    });
    assert_ne!(app.backend.as_ref().unwrap().pid(), pid);
    assert_eq!(app.prompt, "돌아와서 보낼 초안");
    assert_eq!(app.settings.effort, "low");
    assert!(app.can_send());
    assert!(!app.auto_accept);
    assert!(
        app.transcript
            .messages
            .iter()
            .any(|m| m.body.contains("복원한 대화"))
    );
}

#[test]
fn background_project_releases_process_only_after_work_completes() {
    let mut app = connected();
    let a = app.settings.active_project.unwrap();
    let b = app.settings.projects[1].id;
    app.prompt = "hold".into();
    app.submit();
    pump(&mut app, |a| a.busy && a.turn_id.is_some());
    let thread = app.thread_id.clone().unwrap();
    let pid = app.backend.as_ref().unwrap().pid();
    app.switch_project(b);
    assert_eq!(app.sessions[0].backend.as_ref().unwrap().pid(), pid);
    app.sessions[0]
        .backend
        .as_ref()
        .unwrap()
        .send(json!({"method":"test/finish"}))
        .unwrap();
    pump(&mut app, |a| a.sessions[0].backend.is_none());
    assert!(!app.sessions[0].busy);
    assert_eq!(app.sessions[0].thread_id.as_deref(), Some(thread.as_str()));
    assert!(
        app.meter
            .records
            .iter()
            .any(|r| r.thread == thread && r.status == "completed")
    );
    assert!(
        app.desktop
            .sent
            .iter()
            .any(|(title, _)| title.contains("작업 완료"))
    );
    #[cfg(unix)]
    assert_reaped(pid);
    app.switch_project(a);
    pump(&mut app, |a| {
        a.thread_id.as_deref() == Some(thread.as_str()) && !a.loading
    });
    assert!(app.ready && app.backend.is_some());
}

#[test]
fn inactive_project_keeps_unresolved_approvals_and_pending_requests() {
    let mut app = connected();
    app.server_request(
        json!({"id":"review", "method":"item/commandExecution/requestApproval",
        "params":{"command":"echo review","cwd":"/tmp"}}),
    );
    let b = app.settings.projects[1].id;
    let pid = app.backend.as_ref().unwrap().pid();
    app.switch_project(b);
    app.tick_logic(&app.ui_context.clone());
    assert_eq!(app.sessions[0].backend.as_ref().unwrap().pid(), pid);
    app.sessions[0].approvals.clear();
    app.sessions[0].pending.insert(999, Pending::Ignore);
    app.tick_logic(&app.ui_context.clone());
    assert!(app.sessions[0].backend.is_some());
    app.sessions[0].pending.clear();
    pump(&mut app, |a| a.sessions[0].backend.is_none());
    #[cfg(unix)]
    assert_reaped(pid);
}

#[test]
fn connection_buttons_disconnect_reconnect_and_stop_work() {
    let app = fixture();
    let mut h = Harness::builder()
        .with_size(egui::vec2(720.0, 520.0))
        .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
    h.run();
    click(&mut h, "연결");
    pump(h.state_mut(), |a| a.ready && a.pending.is_empty());
    let pid = h.state().backend.as_ref().unwrap().pid();
    h.state_mut().auto_accept = true;
    h.run();
    click(&mut h, "연결 종료");
    assert!(h.state().backend.is_none() && !h.state().auto_accept);
    #[cfg(unix)]
    assert_reaped(pid);
    click(&mut h, "연결");
    pump(h.state_mut(), |a| a.ready && a.pending.is_empty());
    h.state_mut().prompt = "hold".into();
    h.run();
    click(&mut h, "보내기");
    pump(h.state_mut(), |a| a.busy && a.turn_id.is_some());
    h.run();
    click(&mut h, "연결 종료");
    assert!(
        h.state().busy && h.state().backend.is_some(),
        "cannot disconnect during work"
    );
    click(&mut h, "중단");
    pump(h.state_mut(), |a| !a.busy && a.pending.is_empty());
    assert!(h.state().turn_id.is_none());
    assert!(
        h.state()
            .meter
            .records
            .iter()
            .any(|r| r.status == "interrupted")
    );
}

#[test]
fn failed_project_connection_can_be_retried_from_the_selected_project() {
    let mut app = fixture();
    let a = app.settings.active_project.unwrap();
    let binary = app.binary.clone();
    app.binary = "/tmp/codex-mini-backend-that-does-not-exist".into();
    app.switch_project(a);
    assert!(app.error.is_some() && app.backend.is_none() && !app.ready);
    app.binary = binary;
    app.switch_project(a);
    pump(&mut app, |a| a.ready && a.pending.is_empty());
    assert!(app.error.is_none());
    let pid = app.backend.as_ref().unwrap().pid();
    app.switch_project(u64::MAX);
    assert_eq!(app.settings.active_project, Some(a));
    assert_eq!(app.backend.as_ref().unwrap().pid(), pid);
}

#[test]
fn project_editor_validates_cancels_and_reconnects_after_save() {
    let app = connected();
    let pid = app.backend.as_ref().unwrap().pid();
    let mut h = Harness::builder()
        .with_size(egui::vec2(720.0, 520.0))
        .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
    h.run();
    click(&mut h, "프로젝트 편집");
    h.state_mut().project_editor.as_mut().unwrap().cwd =
        "/tmp/codex-mini-folder-that-does-not-exist".into();
    h.run();
    click(&mut h, "저장");
    assert!(h.state().project_error.is_some());
    assert_eq!(h.state().backend.as_ref().unwrap().pid(), pid);
    assert_eq!(h.state().settings.cwd, "/tmp");
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().project_editor.is_none());
    click(&mut h, "프로젝트 편집");
    h.state_mut().project_editor.as_mut().unwrap().instructions = "새 지침".into();
    h.run();
    click(&mut h, "저장");
    assert!(h.state().project_editor.is_none());
    assert_eq!(h.state().settings.active().unwrap().instructions, "새 지침");
    assert_ne!(h.state().backend.as_ref().unwrap().pid(), pid);
    pump(h.state_mut(), |a| a.ready && a.pending.is_empty());
}

#[test]
fn chat_menu_copies_id_and_delete_modal_cancels_or_waits_for_server() {
    let mut app = connected();
    app.prompt = "hello".into();
    app.submit();
    pump(&mut app, |a| {
        a.thread_id.is_some() && !a.busy && !a.loading && a.pending.is_empty()
    });
    let thread = app.thread_id.clone().unwrap();
    let mut h = Harness::builder()
        .with_size(egui::vec2(720.0, 520.0))
        .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
    h.run();
    click(&mut h, "대화");
    let point = h.get_by_label("대화 ID 복사").rect().center();
    h.hover_at(point);
    h.run();
    h.drag_at(point);
    h.step();
    h.input_mut().events.push(egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.step();
    assert!(
        h.output().platform_output.commands.iter().any(
            |command| matches!(command, egui::OutputCommand::CopyText(text) if text == &thread)
        )
    );
    h.run();
    h.get_by_role_and_label(egui::accesskit::Role::Button, "hello")
        .click_secondary();
    h.run();
    click(&mut h, "대화 삭제…");
    h.get_by_label("영구 삭제");
    click(&mut h, "취소");
    assert!(h.state().delete_chat.is_none());
    assert!(h.state().settings.recent.iter().any(|r| r.id == thread));
    h.get_by_role_and_label(egui::accesskit::Role::Button, "hello")
        .click_secondary();
    h.run();
    click(&mut h, "대화 삭제…");
    click(&mut h, "영구 삭제");
    assert!(h.state().loading);
    assert!(h.state().settings.recent.iter().any(|r| r.id == thread));
    pump(h.state_mut(), |a| !a.loading);
    assert!(h.state().settings.recent.iter().all(|r| r.id != thread));
    assert!(h.state().thread_id.is_none());
}

#[test]
fn choosing_history_during_connect_reuses_the_pending_handshake() {
    let mut app = fixture();
    app.settings.remember("saved-thread", "saved chat");
    app.connect(&app.ui_context.clone());
    let pid = app.backend.as_ref().unwrap().pid();
    app.resume(app.settings.recent[0].clone());
    assert_eq!(app.backend.as_ref().unwrap().pid(), pid);
    assert!(app.sessions.is_empty());
    assert_eq!(
        app.resume_after_connect.as_ref().unwrap().id,
        "saved-thread"
    );
    pump(&mut app, |a| {
        a.thread_id.as_deref() == Some("saved-thread") && !a.loading
    });
    assert!(app.error.is_none());
    assert!(app.can_send());
}

#[test]
fn fast_project_switches_settle_with_only_the_active_connection() {
    let mut app = connected();
    let a = app.settings.projects[0].id;
    let b = app.settings.projects[1].id;
    for _ in 0..4 {
        app.switch_project(b);
        app.switch_project(a);
    }
    pump(&mut app, |a| {
        a.ready && a.pending.is_empty() && a.sessions.iter().all(|s| s.backend.is_none())
    });
    assert_eq!(app.settings.active_project, Some(a));
    assert!(app.backend.is_some());
    assert!(app.sessions.iter().all(|s| !s.loading && !s.busy));
    assert!(app.error.is_none());
}

#[test]
fn returning_to_a_project_prefers_the_latest_unsent_chat_over_its_running_chat() {
    let mut app = connected();
    let a = app.settings.projects[0].id;
    let b = app.settings.projects[1].id;
    app.prompt = "hold".into();
    app.submit();
    pump(&mut app, |a| a.busy && a.turn_id.is_some());
    let worker_pid = app.backend.as_ref().unwrap().pid();
    app.new_chat();
    pump(&mut app, |a| a.ready && a.pending.is_empty());
    app.prompt = "새 대화에서 작성 중".into();
    app.switch_project(b);
    pump(&mut app, |a| a.ready && a.pending.is_empty());
    app.switch_project(a);
    pump(&mut app, |a| a.ready && !a.loading);
    assert!(app.thread_id.is_none());
    assert_eq!(app.prompt, "새 대화에서 작성 중");
    assert!(app.sessions.iter().any(|s| {
        s.busy
            && s.backend
                .as_ref()
                .is_some_and(|backend| backend.pid() == worker_pid)
    }));
}

#[test]
fn invalid_project_folder_shows_an_error_and_navigation_recovers() {
    let mut app = connected();
    let a = app.settings.projects[0].id;
    let b = app.settings.projects[1].id;
    app.settings.projects[1].cwd = "/tmp/codex-mini-missing-project-folder".into();
    app.switch_project(b);
    assert!(app.backend.is_none() && !app.ready && !app.loading);
    let mut h = Harness::builder()
        .with_size(egui::vec2(720.0, 520.0))
        .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
    h.run();
    h.get_by_label("존재하는 작업 폴더의 경로를 입력하세요.");
    click(&mut h, "닫기");
    assert!(h.state().error.is_none());
    click(&mut h, "Project A");
    pump(h.state_mut(), |a| a.ready && a.pending.is_empty());
    assert_eq!(h.state().settings.active_project, Some(a));
    assert!(h.state().can_send());
}

#[cfg(feature = "ui-review")]
#[test]
#[ignore = "render project switching at minimum size for visual review"]
fn render_project_switch_review() {
    for theme in [
        crate::preferences::Theme::Light,
        crate::preferences::Theme::Dark,
    ] {
        for zoom in [1.0, 1.5] {
            let mut app = connected();
            app.settings.theme = theme;
            app.prompt = "hello".into();
            app.submit();
            pump(&mut app, |a| {
                a.thread_id.is_some() && !a.busy && a.pending.is_empty()
            });
            let b = app.settings.projects[1].id;
            app.switch_project(b);
            pump(&mut app, |a| a.ready && a.pending.is_empty());
            app.prompt = "전환 뒤 작성 중인 메시지".into();
            let mut h = Harness::builder()
                .with_size(egui::vec2(560.0, 400.0))
                .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
            crate::preferences::configure(&h.ctx, theme);
            install_korean_font(&h.ctx);
            h.ctx.set_zoom_factor(zoom);
            h.run();
            let path = format!("/tmp/codex-mini-project-switch-{theme:?}-{zoom}.png");
            h.render().unwrap().save(&path).unwrap();
            eprintln!("{path}");
            h.key_press(egui::Key::Escape);
            h.run();
            assert!(!h.state().show_projects);
            let path = format!("/tmp/codex-mini-project-connected-{theme:?}-{zoom}.png");
            h.render().unwrap().save(&path).unwrap();
            eprintln!("{path}");
        }
    }
}
