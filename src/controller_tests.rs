use super::*;

#[test]
fn rate_limit_updates_are_live_and_failures_do_not_break_chat() {
    let mut app = chat_fixture();
    app.ready = true;
    app.pending.insert(910, Pending::RateLimits);
    app.process(json!({"id":910,"error":{"code":-32601,"message":"unsupported"}}));
    assert!(app.ready && app.error.is_none());
    assert!(app.rate_limits.error.is_some());
    app.notification(json!({"method":"account/rateLimits/updated","params":{"rateLimits":{"limitId":"codex","primary":{"usedPercent":42,"windowDurationMins":300}}}}));
    assert_eq!(
        app.rate_limits.buckets["codex"]
            .primary
            .as_ref()
            .unwrap()
            .used_percent,
        42.
    );
    assert!(app.rate_limits.error.is_none());
}

#[test]
fn browser_login_refreshes_auth_and_supports_cancel_and_retry() {
    let mut app = chat_fixture();
    app.connect(&Wake::default());
    pump_until(&mut app, |a| a.ready && a.pending.is_empty());
    app.send(json!({"method":"test/sign_out"}));
    pump_until(&mut app, |a| a.needs_auth);
    assert!(!app.can_send());
    app.rpc(
        "account/login/start",
        json!({"type":"chatgpt"}),
        Pending::Login,
    );
    pump_until(&mut app, |a| a.auth_url.is_some());
    assert_eq!(app.login_id.as_deref(), Some("test-login"));
    app.rpc(
        "account/login/cancel",
        json!({"loginId":"test-login"}),
        Pending::CancelLogin,
    );
    pump_until(&mut app, |a| a.login_id.is_none() && a.pending.is_empty());
    assert!(app.needs_auth);
    assert!(app.auth_url.is_none());
    app.rpc(
        "account/login/start",
        json!({"type":"chatgpt"}),
        Pending::Login,
    );
    pump_until(&mut app, |a| a.auth_url.is_some());
    app.send(json!({"method":"test/login_complete"}));
    pump_until(&mut app, |a| !a.needs_auth && a.pending.is_empty());
    assert!(app.auth_url.is_none() && app.login_id.is_none());
    assert!(app.can_send());
    app.disconnect();
    assert!(!app.needs_auth && app.login_id.is_none());
}

#[test]
fn malformed_login_response_does_not_offer_an_unsafe_browser_link() {
    let mut app = chat_fixture();
    app.pending.insert(900, Pending::Login);
    app.process(json!({"id":900,"result":{"authUrl":"file:///sensitive-file"}}));
    assert!(app.auth_url.is_none());
    assert!(app.error.as_ref().unwrap().contains("로그인 주소"));
}

fn chat_fixture() -> MiniApp {
    let mut app = MiniApp::new(
        &Wake::default(),
        Settings {
            cwd: crate::transport::test_cwd().into(),
            ..Default::default()
        },
        format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR")),
    );
    app.models = vec![
        json!({"model":"test-model","isDefault":true,"defaultReasoningEffort":"medium","supportedReasoningEfforts":[{"reasoningEffort":"low"},{"reasoningEffort":"medium"},{"reasoningEffort":"high"}]}),
        json!({"model":"other-model","defaultReasoningEffort":"low","supportedReasoningEfforts":[{"reasoningEffort":"low"}]}),
    ];
    app.settings.model = "test-model".into();
    app
}

#[test]
fn serial_queue_waits_for_background_turn_and_preserves_a_new_draft() {
    let mut app = live_worker();
    app.new_chat();
    pump_until(&mut app, |a| a.ready);
    app.prompt = "queued task".into();
    app.submit();
    assert!(!app.busy);
    assert_eq!(app.queued_job.as_ref().unwrap().prompt, "queued task");
    assert_eq!(app.running_count(), 1);
    app.prompt = "newer draft".into();
    app.sessions[0]
        .backend
        .as_ref()
        .unwrap()
        .send(json!({"method":"test/finish"}))
        .unwrap();
    pump_until(&mut app, |a| {
        a.queued_job.is_none() && a.thread_id.is_some()
    });
    pump_until(&mut app, |a| !a.busy && !a.loading);
    assert_eq!(app.prompt, "newer draft");
    assert!(
        !app.transcript
            .messages
            .iter()
            .any(|m| m.title == "전송 실패")
    );
    assert!(
        app.transcript
            .messages
            .iter()
            .any(|m| m.body == "queued task")
    );
}

#[test]
fn new_chat_parks_queued_work_and_disconnect_restores_text() {
    let mut app = chat_fixture();
    app.queued_job = Some(QueuedJob {
        order: 1,
        prompt: "waiting".into(),
        model: "m".into(),
        effort: "low".into(),
    });
    app.new_chat();
    assert!(app.queued_job.is_none());
    assert_eq!(
        app.sessions[0].queued_job.as_ref().unwrap().prompt,
        "waiting"
    );
    app.activate_session(0);
    app.disconnect();
    assert_eq!(app.prompt, "waiting");
    assert!(app.queued_job.is_none());
}

#[test]
fn history_pages_prepend_deduplicate_and_keep_a_bounded_window() {
    let mut app = chat_fixture();
    app.transcript.put("new", "CODEX", "newest", false, false);
    app.pending.insert(1, Pending::History { older: true });
    app.process(json!({"id":1,"result":{"data":[{"items":[{"id":"old","type":"agentMessage","text":"older"},{"id":"new","type":"agentMessage","text":"duplicate"}]}],"nextCursor":"page-2"}}));
    assert_eq!(app.history_cursor.as_deref(), Some("page-2"));
    assert_eq!(app.transcript.messages[0].body, "older");
    assert_eq!(app.transcript.messages[1].body, "newest");
    for i in 0..160 {
        app.transcript
            .put(&format!("m{i}"), "CODEX", "body", false, false);
    }
    app.pending.insert(2, Pending::History { older: true });
    app.process(json!({"id":2,"result":{"data":[{"items":[{"id":"earliest","type":"agentMessage","text":"keep this page"}]}],"nextCursor":null}}));
    assert_eq!(app.transcript.messages[0].id, "earliest");
    assert!(app.transcript.messages.len() <= 160);
    assert!(app.history_cursor.is_none());
}

#[test]
fn model_pages_merge_without_duplicates() {
    let mut app = chat_fixture();
    app.models.clear();
    app.pending.insert(1, Pending::Models);
    app.process(json!({"id":1,"result":{"data":[{"model":"a"}],"nextCursor":null}}));
    app.pending.insert(2, Pending::Models);
    app.process(json!({"id":2,"result":{"data":[{"model":"a"},{"model":"b"}],"nextCursor":null}}));
    assert_eq!(app.models.len(), 2);
}

#[test]
fn auto_approval_never_invents_missing_permissions_or_accepts_truncated_previews() {
    let mut app = chat_fixture();
    app.server_request(json!({"id":1,"method":"item/permissions/requestApproval","params":{"reason":"permission needed"}}));
    assert!(automatic_approval(app.approvals.front().unwrap()).is_none());
    app.approvals.clear();
    app.server_request(json!({"id":2,"method":"item/commandExecution/requestApproval","params":{"command":"x".repeat(state::MAX_BODY+1)}}));
    assert!(automatic_approval(app.approvals.front().unwrap()).is_none());
    app.approvals.clear();
    app.server_request(json!({"id":3,"method":"item/commandExecution/requestApproval","params":{"command":"cargo test","availableDecisions":["decline"]}}));
    assert!(automatic_approval(app.approvals.front().unwrap()).is_none());
}

#[test]
fn followup_failure_keeps_the_newer_draft_and_reconciles_local_echo() {
    let mut app = chat_fixture();
    app.backend = Some(Backend::start_command(crate::transport::mock_command(), || {}).unwrap());
    app.ready = true;
    hold_turn(&mut app);
    app.prompt = "reject".into();
    app.submit();
    app.prompt = "newer draft".into();
    receive_until(&mut app, |a| a.error.is_some());
    assert_eq!(app.prompt, "newer draft");
    assert!(app.busy);
    assert!(
        app.transcript
            .messages
            .iter()
            .any(|m| m.body == "reject" && m.tool)
    );
    assert!(
        !app.transcript
            .messages
            .iter()
            .any(|m| m.body == "reject" && !m.tool)
    );
}

#[test]
fn reasoning_effort_uses_the_models_supported_default() {
    let mut app = chat_fixture();
    app.settings.effort = "unsupported".into();
    assert_eq!(app.effective_effort(), "medium");
    app.settings.effort.clear();
    app.active_efficiency.effort = "high".into();
    assert_eq!(app.effective_effort(), "high");
}

#[test]
fn manual_compaction_waits_for_the_same_execution_budget() {
    let mut app = live_worker();
    app.new_chat();
    pump_until(&mut app, |a| a.ready);
    app.thread_id = Some("mock-thread".into());
    app.compact();
    assert!(app.compact_due && !app.busy);
    assert_eq!(app.running_count(), 1);
    app.sessions[0]
        .backend
        .as_ref()
        .unwrap()
        .send(json!({"method":"test/finish"}))
        .unwrap();
    pump_until(&mut app, |a| {
        !a.sessions[0].busy && !a.compact_due && !a.busy
    });
    assert_eq!(app.status, "컨텍스트 압축 완료");
}

#[test]
fn quitting_finalizes_background_usage_and_reaps_every_owned_backend() {
    let mut app = live_worker();
    app.new_chat();
    pump_until(&mut app, |a| a.ready);
    app.shutdown();
    assert!(app.backend.is_none() && app.sessions.is_empty());
    assert!(
        app.meter
            .records
            .iter()
            .any(|r| r.thread == "worker-a" && r.status == "disconnected")
    );
}
#[test]
fn empty_instructions_allow_new_and_resumed_chats() {
    let mut app = chat_fixture();
    app.settings.projects[0].instructions = " \n ".into();
    app.backend = Some(
        Backend::start(
            &format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR")),
            crate::transport::test_cwd(),
            || {},
        )
        .unwrap(),
    );
    app.ready = true;
    let mut params = json!({});
    app.project_instructions(&mut params);
    assert!(params.get("developerInstructions").is_none());
    app.prompt = "run".into();
    app.submit();
    assert!(!app.pending.is_empty());
    assert!(app.loading);
    assert_eq!(app.queued_prompt.as_deref(), Some("run"));
    assert!(app.project_editor.is_none());
    app.loading = false;
    app.pending.clear();
    app.settings.remember("old", "old");
    app.resume(app.settings.recent[0].clone());
    assert!(app.loading);
    assert!(!app.pending.is_empty());
    assert!(app.project_editor.is_none());
}

#[test]
fn automatic_compaction_uses_last_usage_and_finishes_via_notifications() {
    let mut app = chat_fixture();
    let command = crate::transport::mock_command();
    app.backend = Some(Backend::start_command(command, || {}).unwrap());
    app.ready = true;
    app.thread_id = Some("mock-thread".into());
    app.busy = true;
    let usage = |n| json!({"method":"thread/tokenUsage/updated","params":{"threadId":"mock-thread","tokenUsage":{"modelContextWindow":1000,"total":{"totalTokens":999999},"last":{"totalTokens":n}}}});
    app.notification(usage(799));
    assert!(!app.compact_due);
    app.notification(usage(800));
    assert!(app.compact_due);
    assert!(!app.compacting);
    app.notification(json!({"method":"turn/completed","params":{"threadId":"mock-thread","turn":{"id":"normal","status":"completed"}}}));
    assert!(app.compacting && app.busy);
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.compacting && Instant::now() < deadline {
        let event = app
            .backend
            .as_ref()
            .unwrap()
            .incoming
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        if let Event::Message(v) = event {
            app.process(v);
        }
    }
    assert!(!app.compacting && !app.busy && !app.compact_due);
    assert_eq!(app.status, "컨텍스트 압축 완료");
    app.new_chat();
    assert!(!app.compact_due);
}

#[test]
fn followup_questions_notify_even_with_pending_requests_and_auto_accept() {
    for language in [Language::Korean, Language::English] {
        let mut app = chat_fixture();
        app.settings.language = language;
        app.hidden = true;
        app.server_request(
            json!({"id":"pending","method":"item/commandExecution/requestApproval","params":{}}),
        );
        app.desktop.sent.clear();
        app.auto_accept = true;
        for id in ["q1", "q2"] {
            app.server_request(json!({"id":id,"method":"item/tool/requestUserInput","params":{"questions":[{"id":"q","question":"Private question"}]}}));
        }
        assert_eq!(app.desktop.sent.len(), 2);
        assert_eq!(
            app.desktop.sent[0].0,
            language.text("Codex Mini · 답변 필요")
        );
        assert!(!app.desktop.sent[0].1.contains("Private question"));
        assert_eq!(app.approvals.len(), 3);
        app.settings.notifications = false;
        app.server_request(
            json!({"id":"quiet","method":"item/tool/requestUserInput","params":{"questions":[]}}),
        );
        assert_eq!(app.desktop.sent.len(), 2);
    }
}

#[test]
fn failed_auto_accept_send_keeps_request_for_retry() {
    let mut app = chat_fixture();
    app.auto_accept = true;
    app.server_request(json!({"id":"retry","method":"item/commandExecution/requestApproval","params":{"command":"ls"}}));
    assert_eq!(app.approvals.len(), 1);
    assert!(app.error.is_some());
}

#[test]
fn completion_notifies_once_while_hidden_and_preference_is_respected() {
    let mut app = chat_fixture();
    app.thread_id = Some("thread".into());
    app.busy = true;
    app.hidden = true;
    let completed = json!({"method":"turn/completed","params":{"threadId":"thread","turn":{"id":"work","status":"completed"}}});
    app.notification(completed.clone());
    app.notification(completed);
    assert_eq!(app.desktop.sent.len(), 1);
    assert!(app.desktop.sent[0].0.contains("작업 완료"));
    assert!(app.hidden);
    app.busy = true;
    app.settings.notifications = false;
    app.notification(json!({"method":"turn/completed","params":{"threadId":"thread","turn":{"id":"muted","status":"completed"}}}));
    assert_eq!(app.desktop.sent.len(), 1);
    app.settings.notifications = true;
    app.busy = true;
    app.compacting = true;
    app.notification(json!({"method":"turn/completed","params":{"threadId":"thread","turn":{"id":"compaction","status":"completed"}}}));
    assert_eq!(
        app.desktop.sent.len(),
        1,
        "compaction isn't user work completion"
    );
    app.busy = true;
    app.server_request(json!({"id":"permission","method":"item/commandExecution/requestApproval","params":{"command":"echo hello"}}));
    assert_eq!(app.desktop.sent.len(), 2);
    assert!(app.desktop.sent[1].0.contains("확인 필요"));
}

fn receive_until(app: &mut MiniApp, done: impl Fn(&MiniApp) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while !done(app) {
        assert!(
            Instant::now() < deadline,
            "backend did not reach expected state"
        );
        match app
            .backend
            .as_ref()
            .unwrap()
            .incoming
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
        {
            Event::Message(message) => app.process(message),
            other => panic!("unexpected backend event: {other:?}"),
        }
    }
}

fn hold_turn(app: &mut MiniApp) {
    app.thread_id = Some("mock-thread".into());
    app.start_turn("hold".into());
    assert_eq!(app.transcript.messages.len(), 1, "immediate local echo");
    receive_until(app, |app| {
        app.turn_id.is_some() && app.transcript.messages.iter().any(|m| m.id == "user-1")
    });
    assert_eq!(
        app.transcript.messages.len(),
        1,
        "server echo replaces local echo"
    );
}

fn live_worker() -> MiniApp {
    let mut app = chat_fixture();
    app.binary = format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR"));
    app.settings.projects[0].cwd = crate::transport::test_cwd().into();
    app.settings.cwd = crate::transport::test_cwd().into();
    let mut command = crate::transport::mock_command();
    command.env("CODEX_MINI_MOCK_THREAD", "worker-a");
    app.backend = Some(Backend::start_command(command, || {}).unwrap());
    app.ready = true;
    app.prompt = "hold".into();
    app.submit();
    pump_until(&mut app, |app| app.busy && app.turn_id.is_some());
    app
}

fn pump_until(app: &mut MiniApp, ready: impl Fn(&MiniApp) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !ready(app) {
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

#[test]
fn new_chat_keeps_worker_and_routes_background_questions_and_completion() {
    let mut app = live_worker();
    app.settings.max_concurrent_tasks = 2;
    let first_pid = app.backend.as_ref().unwrap().pid();
    app.auto_accept = true;
    app.prompt = "worker A draft".into();
    app.new_chat();
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.sessions[0].backend.as_ref().unwrap().pid(), first_pid);
    assert!(app.sessions[0].busy && app.sessions[0].auto_accept);
    assert!(!app.auto_accept && app.prompt.is_empty());
    assert_ne!(app.backend.as_ref().unwrap().pid(), first_pid);
    pump_until(&mut app, |app| app.ready);
    app.prompt = "hold".into();
    app.submit();
    pump_until(&mut app, |app| app.busy && app.turn_id.is_some());
    let second_pid = app.backend.as_ref().unwrap().pid();
    app.sessions[0]
        .backend
        .as_ref()
        .unwrap()
        .send(json!({"method":"test/question"}))
        .unwrap();
    pump_until(&mut app, |app| !app.sessions[0].approvals.is_empty());
    assert!(app.approvals.is_empty());
    assert!(
        app.desktop
            .sent
            .iter()
            .any(|(title, _)| title.contains("답변 필요"))
    );
    app.sessions[0]
        .backend
        .as_ref()
        .unwrap()
        .send(json!({"method":"test/finish"}))
        .unwrap();
    pump_until(&mut app, |app| !app.sessions[0].busy);
    assert!(app.busy);
    assert!(
        app.desktop
            .sent
            .iter()
            .any(|(title, _)| title.contains("작업 완료"))
    );
    assert!(
        app.meter
            .records
            .iter()
            .any(|r| r.thread == "worker-a" && r.status == "completed")
    );
    assert!(
        !app.transcript
            .messages
            .iter()
            .any(|m| m.body.contains("Background reply"))
    );
    let first = app
        .settings
        .recent
        .iter()
        .find(|r| r.id == "worker-a")
        .unwrap()
        .clone();
    assert!(
        app.sessions[0].backend.is_none(),
        "completed background conversations release their CLI process"
    );
    app.resume(first);
    assert_ne!(app.backend.as_ref().unwrap().pid(), first_pid);
    assert_eq!(app.prompt, "worker A draft");
    assert_eq!(app.models[0]["model"], "test-model");
    assert!(
        app.transcript
            .messages
            .iter()
            .any(|m| m.body.contains("Background reply"))
    );
    assert!(
        app.approvals.is_empty(),
        "completed turns clear their old questions"
    );
    assert_eq!(app.sessions[0].backend.as_ref().unwrap().pid(), second_pid);
    assert!(app.sessions[0].busy);
}

#[test]
fn switching_projects_restores_the_original_live_connection() {
    let mut app = live_worker();
    let first_project = app.settings.active_project.unwrap();
    let first_pid = app.backend.as_ref().unwrap().pid();
    let second_project = app
        .settings
        .add_project(
            "Second project",
            crate::transport::test_cwd(),
            "Second instructions",
        )
        .unwrap();
    app.switch_project(second_project);
    assert_eq!(app.settings.active_project, Some(second_project));
    assert!(app.sessions[0].busy);
    pump_until(&mut app, |app| app.ready);
    app.prompt = "second project draft".into();
    app.switch_project(first_project);
    assert_eq!(app.backend.as_ref().unwrap().pid(), first_pid);
    assert!(app.busy);
    app.switch_project(second_project);
    assert_eq!(app.prompt, "second project draft");
    assert!(
        app.sessions
            .iter()
            .any(|s| s.project_id == Some(first_project) && s.busy)
    );
}

#[test]
fn pending_thread_start_can_complete_after_navigating_away() {
    let mut app = chat_fixture();
    app.binary = format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR"));
    app.settings.cwd = crate::transport::test_cwd().into();
    app.settings.projects[0].cwd = crate::transport::test_cwd().into();
    app.backend = Some(Backend::start(&app.binary, crate::transport::test_cwd(), || {}).unwrap());
    app.ready = true;
    app.prompt = "hold".into();
    app.submit();
    assert!(app.loading && app.thread_id.is_none());
    let second = app
        .settings
        .add_project("Second", crate::transport::test_cwd(), "")
        .unwrap();
    app.switch_project(second);
    pump_until(&mut app, |app| {
        app.sessions[0].busy && app.sessions[0].turn_id.is_some()
    });
    assert_eq!(app.settings.active_project, Some(second));
    assert!(app.transcript.messages.is_empty());
    assert!(
        app.sessions[0]
            .transcript
            .messages
            .iter()
            .any(|m| m.body == "hold")
    );
    assert!(
        app.settings
            .recent
            .iter()
            .any(|r| r.id == "mock-thread" && r.project_id == app.sessions[0].project_id)
    );
    assert!(!app.deferred_save);
}

#[test]
fn disk_chat_resume_waits_for_its_new_backend_handshake() {
    let mut app = live_worker();
    app.settings.remember("disk-chat", "previous chat");
    let recent = app
        .settings
        .recent
        .iter()
        .find(|r| r.id == "disk-chat")
        .unwrap()
        .clone();
    app.resume(recent);
    assert!(app.resume_after_connect.is_some());
    assert!(app.sessions[0].busy);
    pump_until(&mut app, |app| {
        app.thread_id.as_deref() == Some("disk-chat") && !app.loading
    });
    assert!(app.resume_after_connect.is_none());
    assert!(
        app.transcript
            .messages
            .iter()
            .any(|m| m.body.contains("복원한 대화"))
    );
    assert!(app.sessions[0].busy);
}

#[test]
fn approval_preview_shows_scope_and_flags_truncated_content() {
    let (text, complete) = approval_preview(
        &json!({"command":"ls", "cwd":crate::transport::test_cwd(), "threadId":"private-id", "additionalPermissions":{"network":true}}),
        "",
        Language::Korean,
    );
    assert!(complete);
    assert!(
        text.contains("ls")
            && text.contains(crate::transport::test_cwd())
            && text.contains("network")
    );
    assert!(!text.contains("private-id"));
    assert!(
        !approval_preview(
            &json!({"command":"x".repeat(state::MAX_BODY+1)}),
            "",
            Language::Korean
        )
        .1
    );
}
