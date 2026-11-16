use crate::i18n::Language;
use crate::{
    state::{self, Project, Recent, Settings, Transcript, clipped, string},
    transport::{Backend, Event},
};
use eframe::egui::{self, RichText};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

#[derive(Clone)]
enum Pending {
    Initialize,
    Delete(String),
    Compact,
    Account,
    Models,
    Start,
    Resume,
    History,
    Turn,
    Steer { local_id: String, prompt: String },
    Interrupt,
    Login,
    Ignore,
    Skills,
    SkillSaved,
    LocalSkills,
    PrepareThread { params: Value, resume: bool },
}

enum ChatAction {
    Move(String, u64),
    Delete(Recent),
    Compact,
}

struct Approval {
    id: Value,
    method: String,
    params: Value,
    preview: String,
    preview_complete: bool,
    previous: String,
    preview_language: Language,
    answers: HashMap<String, String>,
}

// Only conversation state is swapped. Shell services and saved preferences stay shared.
#[derive(Default)]
struct Session {
    project_id: Option<u64>,
    cwd: String,
    model: String,
    models: Vec<Value>,
    account: String,
    effort: String,
    backend: Option<Backend>,
    pending: HashMap<u64, Pending>,
    next_id: u64,
    ready: bool,
    busy: bool,
    loading: bool,
    thread_id: Option<String>,
    turn_id: Option<String>,
    transcript: Transcript,
    approvals: VecDeque<Approval>,
    auto_accept: bool,
    diagnostics: VecDeque<String>,
    prompt: String,
    prompt_composing: bool,
    queued_prompt: Option<String>,
    submitted_prompt: String,
    submitted_message_id: Option<String>,
    status: String,
    error: Option<String>,
    auth_url: Option<String>,
    resume_after_connect: Option<Recent>,
    started: Option<Instant>,
    active_efficiency: crate::efficiency::Options,
    local_skills: Vec<Value>,
    mcp_draft: String,
    compacting: bool,
    compact_due: bool,
    completed_turns: VecDeque<String>,
    meter: crate::usage::Meter,
}

impl Session {
    fn swap(&mut self, app: &mut MiniApp) {
        std::mem::swap(&mut self.project_id, &mut app.settings.active_project);
        std::mem::swap(&mut self.cwd, &mut app.settings.cwd);
        std::mem::swap(&mut self.model, &mut app.settings.model);
        std::mem::swap(&mut self.models, &mut app.models);
        std::mem::swap(&mut self.account, &mut app.account);
        std::mem::swap(&mut self.effort, &mut app.settings.effort);
        std::mem::swap(&mut self.backend, &mut app.backend);
        std::mem::swap(&mut self.pending, &mut app.pending);
        std::mem::swap(&mut self.next_id, &mut app.next_id);
        std::mem::swap(&mut self.ready, &mut app.ready);
        std::mem::swap(&mut self.busy, &mut app.busy);
        std::mem::swap(&mut self.loading, &mut app.loading);
        std::mem::swap(&mut self.thread_id, &mut app.thread_id);
        std::mem::swap(&mut self.turn_id, &mut app.turn_id);
        std::mem::swap(&mut self.transcript, &mut app.transcript);
        std::mem::swap(&mut self.approvals, &mut app.approvals);
        std::mem::swap(&mut self.auto_accept, &mut app.auto_accept);
        std::mem::swap(&mut self.diagnostics, &mut app.diagnostics);
        std::mem::swap(&mut self.prompt, &mut app.prompt);
        std::mem::swap(&mut self.prompt_composing, &mut app.prompt_composing);
        std::mem::swap(&mut self.queued_prompt, &mut app.queued_prompt);
        std::mem::swap(&mut self.submitted_prompt, &mut app.submitted_prompt);
        std::mem::swap(
            &mut self.submitted_message_id,
            &mut app.submitted_message_id,
        );
        std::mem::swap(&mut self.status, &mut app.status);
        std::mem::swap(&mut self.error, &mut app.error);
        std::mem::swap(&mut self.auth_url, &mut app.auth_url);
        std::mem::swap(
            &mut self.resume_after_connect,
            &mut app.resume_after_connect,
        );
        std::mem::swap(&mut self.started, &mut app.started);
        std::mem::swap(&mut self.active_efficiency, &mut app.active_efficiency);
        std::mem::swap(&mut self.local_skills, &mut app.local_skills);
        std::mem::swap(&mut self.mcp_draft, &mut app.mcp_draft);
        std::mem::swap(&mut self.compacting, &mut app.compacting);
        std::mem::swap(&mut self.compact_due, &mut app.compact_due);
        std::mem::swap(&mut self.completed_turns, &mut app.completed_turns);
        self.meter.swap_current(&mut app.meter);
    }
}

pub struct MiniApp {
    pub settings: Settings,
    sessions: Vec<Session>,
    background_processing: bool,
    deferred_save: bool,
    discarded_threads: Vec<String>,
    binary: String,
    backend: Option<Backend>,
    pending: HashMap<u64, Pending>,
    next_id: u64,
    ready: bool,
    busy: bool,
    loading: bool,
    thread_id: Option<String>,
    turn_id: Option<String>,
    transcript: Transcript,
    models: Vec<Value>,
    approvals: VecDeque<Approval>,
    auto_accept: bool,
    diagnostics: VecDeque<String>,
    prompt: String,
    prompt_composing: bool,
    queued_prompt: Option<String>,
    submitted_prompt: String,
    submitted_message_id: Option<String>,
    status: String,
    error: Option<String>,
    account: String,
    auth_url: Option<String>,
    resume_after_connect: Option<Recent>,
    show_settings: bool,
    show_diagnostics: bool,
    started: Option<Instant>,
    serial: u64,
    show_projects: bool,
    project_editor: Option<Project>,
    project_error: Option<String>,
    drafts: HashMap<u64, String>,
    global: crate::global::GlobalUi,
    ui_context: egui::Context,
    meter: crate::usage::Meter,
    journal: Option<crate::usage::Journal>,
    usage_error: Option<String>,
    show_usage: bool,
    active_efficiency: crate::efficiency::Options,
    rtk_path: Option<std::path::PathBuf>,
    local_skills: Vec<Value>,
    mcp_draft: String,
    delete_chat: Option<Recent>,
    compacting: bool,
    compact_due: bool,
    composer_height: f32,
    desktop: crate::desktop::Desktop,
    hidden: bool,
    exiting: bool,
    show_exit_confirmation: bool,
    completed_turns: VecDeque<String>,
}

impl MiniApp {
    pub fn new(cc: &eframe::CreationContext<'_>, mut settings: Settings, binary: String) -> Self {
        settings.prepare_projects();
        let ctx = &cc.egui_ctx;
        crate::preferences::configure(ctx, settings.theme);
        #[cfg(not(test))]
        install_korean_font(ctx);
        #[cfg(not(test))]
        let journal = Settings::path()
            .and_then(|p| p.parent().map(|p| p.join("usage")))
            .map(|path| {
                let ctx = ctx.clone();
                crate::usage::Journal::start(path, move || ctx.request_repaint())
            });
        #[cfg(test)]
        let journal = None;
        Self {
            sessions: Vec::new(),
            background_processing: false,
            deferred_save: false,
            discarded_threads: Vec::new(),
            desktop: crate::desktop::Desktop::new(ctx, settings.language),
            hidden: false,
            exiting: false,
            show_exit_confirmation: false,
            completed_turns: VecDeque::new(),
            meter: Default::default(),
            journal,
            usage_error: None,
            show_usage: false,
            active_efficiency: Default::default(),
            rtk_path: crate::efficiency::bundled_rtk(),
            local_skills: Vec::new(),
            mcp_draft: String::new(),
            delete_chat: None,
            compacting: false,
            compact_due: false,
            composer_height: 0.0,
            settings,
            binary,
            backend: None,
            pending: HashMap::new(),
            next_id: 1,
            ready: false,
            busy: false,
            loading: false,
            thread_id: None,
            turn_id: None,
            transcript: Transcript::default(),
            models: Vec::new(),
            approvals: VecDeque::new(),
            auto_accept: false,
            diagnostics: VecDeque::new(),
            prompt: String::new(),
            prompt_composing: false,
            queued_prompt: None,
            submitted_prompt: String::new(),
            submitted_message_id: None,
            status: "연결 전 · 대기 중".into(),
            error: None,
            account: String::new(),
            auth_url: None,
            resume_after_connect: None,
            show_settings: false,
            show_diagnostics: false,
            started: None,
            serial: 0,
            show_projects: cc.egui_ctx.content_rect().width() >= 620.0,
            project_editor: None,
            project_error: None,
            drafts: HashMap::new(),
            global: crate::global::GlobalUi::default(),
            ui_context: ctx.clone(),
        }
    }

    fn send(&mut self, value: Value) -> bool {
        let result = self
            .backend
            .as_ref()
            .ok_or_else(|| "Codex에 먼저 연결하세요.".to_owned())
            .and_then(|b| b.send(value));
        if let Err(e) = result {
            self.error = Some(e);
            false
        } else {
            true
        }
    }

    fn rpc(&mut self, method: &str, params: Value, pending: Pending) -> bool {
        let id = self.next_id;
        self.next_id += 1;
        if self.send(json!({"id":id,"method":method,"params":params})) {
            self.pending.insert(id, pending);
            true
        } else {
            false
        }
    }

    fn save(&mut self) {
        if self.background_processing {
            self.deferred_save = true;
            return;
        }
        self.settings.codex_binary = self.binary.clone();
        #[cfg(not(test))]
        if let Err(e) = self.settings.save() {
            self.error = Some(format!("설정 저장 실패: {e}"));
        }
    }

    fn connect(&mut self, ctx: &egui::Context) {
        if self.backend.is_some() {
            return;
        }
        let path = match std::fs::canonicalize(&self.settings.cwd) {
            Ok(p) if p.is_dir() => p,
            _ => {
                self.error = Some("존재하는 작업 폴더의 경로를 입력하세요.".into());
                return;
            }
        };
        self.settings.cwd = path.display().to_string();
        let wake = ctx.clone();
        match Backend::start(&self.binary, &self.settings.cwd, move || {
            wake.request_repaint_after(Duration::from_millis(50))
        }) {
            Ok(backend) => {
                // Suspended project conversations resume only after the new handshake.
                if let Some(id) = self.thread_id.take() {
                    self.resume_after_connect = Some(Recent {
                        title: self
                            .settings
                            .recent
                            .iter()
                            .find(|r| r.id == id)
                            .map(|r| r.title.clone())
                            .unwrap_or_default(),
                        id,
                        cwd: self.settings.cwd.clone(),
                        project_id: self.settings.active_project,
                        model: self.settings.model.clone(),
                        effort: self.settings.effort.clone(),
                        efficiency: self.active_efficiency.clone(),
                    });
                }
                self.backend = Some(backend);
                self.ready = false;
                self.status = "Codex 연결 중…".into();
                self.error = None;
                self.loading = true;
                self.rpc("initialize",json!({"clientInfo":{"name":"codex_mini","title":"Codex Mini","version":env!("CARGO_PKG_VERSION")}}),Pending::Initialize);
                self.save();
            }
            Err(e) => {
                self.error = Some(format!(
                    "Codex를 실행하지 못했습니다: {e}\n터미널에서 codex login을 먼저 실행하세요."
                ))
            }
        }
    }

    fn disconnect(&mut self) {
        self.compacting = false;
        self.compact_due = false;
        self.finish_usage("disconnected");
        self.auto_accept = false;
        self.backend.take();
        self.pending.clear();
        self.approvals.clear();
        self.ready = false;
        self.loading = false;
        self.busy = false;
        self.turn_id = None;
        self.thread_id = None;
        self.queued_prompt = None;
        self.account.clear();
        self.auth_url = None;
        self.resume_after_connect = None;
        self.status = "연결 종료 · Codex 프로세스 해제".into();
    }

    fn has_conversation(&self) -> bool {
        self.thread_id.is_some()
            || self.busy
            || self.loading
            || !self.approvals.is_empty()
            || !self.prompt.is_empty()
    }

    fn suspend_idle_connection(&mut self) {
        if self.backend.is_none()
            || self.busy
            || self.loading
            || self.compacting
            || !self.approvals.is_empty()
            || !self.pending.is_empty()
        {
            return;
        }
        // Keep the chat and draft in memory, but release the inactive project's process.
        self.backend.take();
        self.ready = false;
        self.auto_accept = false;
        self.account.clear();
        self.auth_url = None;
        self.status = "연결 종료 · Codex 프로세스 해제".into();
    }

    fn park_conversation(&mut self) -> bool {
        let connected = self.backend.is_some();
        let mut fresh = Session {
            project_id: self.settings.active_project,
            cwd: self.settings.cwd.clone(),
            model: self.settings.model.clone(),
            models: self.models.clone(),
            account: self.account.clone(),
            effort: self.settings.effort.clone(),
            next_id: 1,
            status: "새 대화".into(),
            ..Default::default()
        };
        fresh.swap(self);
        self.sessions.push(fresh);
        connected
    }

    fn activate_session(&mut self, index: usize) {
        if index >= self.sessions.len() {
            return;
        }
        let mut selected = self.sessions.remove(index);
        if let Some(project) = self.settings.active_project {
            self.drafts.insert(project, self.prompt.clone());
        }
        if selected.project_id != self.settings.active_project {
            self.suspend_idle_connection();
        }
        if self.has_conversation() {
            self.park_conversation();
        } else {
            self.disconnect();
        }
        selected.swap(self);
        self.prompt_composing = false;
        if self.backend.is_none() {
            self.connect(&self.ui_context.clone());
        }
        self.save();
    }

    fn new_chat(&mut self) {
        if self.busy || self.loading || !self.approvals.is_empty() {
            if self.park_conversation() {
                self.connect(&self.ui_context.clone());
            }
            return;
        }
        self.compacting = false;
        self.compact_due = false;
        if let Some(id) = self.thread_id.take() {
            self.rpc(
                "thread/unsubscribe",
                json!({"threadId":id}),
                Pending::Ignore,
            );
        }
        self.transcript = Transcript::default();
        self.active_efficiency = Default::default();
        self.meter.attach("", false);
        self.approvals.clear();
        self.turn_id = None;
        self.error = None;
        self.status = "새 대화".into();
    }

    fn can_send(&self) -> bool {
        self.ready
            && !self.loading
            && !self.compacting
            && (!self.busy || (self.thread_id.is_some() && self.turn_id.is_some()))
    }

    fn user_message_id(&mut self) -> String {
        self.serial += 1;
        format!("user-{}-{}", std::process::id(), self.serial)
    }

    fn submit(&mut self) {
        if !self.can_send() || self.prompt_composing || self.prompt.trim().is_empty() {
            return;
        }
        if self.prompt.len() > 128 * 1024 {
            self.error = Some("메시지는 128 KiB 이하로 입력하세요.".into());
            return;
        }
        let prompt = self.prompt.clone();
        self.error = None;
        if self.busy {
            let local_id = self.user_message_id();
            if self.rpc(
                "turn/steer",
                json!({
                    "threadId": self.thread_id, "expectedTurnId": self.turn_id,
                    "input": [{"type":"text", "text":prompt}],
                    "clientUserMessageId": local_id,
                }),
                Pending::Steer {
                    local_id: local_id.clone(),
                    prompt: prompt.clone(),
                },
            ) {
                self.transcript.put(&local_id, "나", &prompt, false, false);
                self.prompt.clear();
            }
        } else if self.thread_id.is_none() {
            let mut params = json!({"cwd":self.settings.cwd});
            if !self.settings.model.is_empty() {
                params["model"] = self.settings.model.clone().into();
            }
            self.active_efficiency = self
                .settings
                .active()
                .map(|p| p.efficiency.clone())
                .unwrap_or_default();
            self.project_instructions(&mut params);
            if self.prepare_thread(params, false) {
                self.queued_prompt = Some(prompt);
                self.loading = true;
                self.status = "대화 준비 중…".into();
            }
        } else {
            self.start_turn(prompt);
        }
    }

    fn start_turn(&mut self, prompt: String) {
        let Some(thread) = self.thread_id.clone() else {
            return;
        };
        let local_id = self.user_message_id();
        let mut params = state::turn_params(
            &thread,
            &prompt,
            &self.settings.model,
            &self.effective_effort(),
        );
        params["clientUserMessageId"] = local_id.clone().into();
        if self.rpc("turn/start", params, Pending::Turn) {
            let project = self
                .settings
                .active()
                .map(|p| p.name.clone())
                .unwrap_or_default();
            self.meter.begin(
                &project,
                &self.settings.model,
                &self.effective_effort(),
                &self.active_efficiency.label(),
            );
            self.submitted_prompt = prompt.clone();
            self.submitted_message_id = Some(local_id.clone());
            self.transcript.put(&local_id, "나", &prompt, false, false);
            if self.prompt == prompt {
                self.prompt.clear();
            }
            self.busy = true;
            self.started = Some(Instant::now());
            self.status = "작업 중".into();
            self.settings.remember(&thread, &prompt);
            if let Some(recent) = self.settings.recent.iter_mut().find(|r| r.id == thread) {
                recent.efficiency = self.active_efficiency.clone();
            }
            self.save();
        }
    }

    fn project_instructions(&self, params: &mut Value) {
        if let Some(project) = self
            .settings
            .active()
            .filter(|p| !p.instructions.trim().is_empty())
        {
            params["developerInstructions"] = project.instructions.clone().into();
        }
    }

    fn switch_project(&mut self, id: u64) {
        if self.settings.active_project == Some(id) {
            self.connect(&self.ui_context.clone());
            return;
        }
        let Some(project) = self.settings.projects.iter().find(|p| p.id == id) else {
            return;
        };
        let cwd = project.cwd.clone();
        if let Some(index) = self.sessions.iter().rposition(|s| s.project_id == Some(id)) {
            self.activate_session(index);
            return;
        }
        if let Some(previous) = self.settings.active_project {
            self.drafts.insert(previous, self.prompt.clone());
        }
        self.suspend_idle_connection();
        if self.has_conversation() {
            self.park_conversation();
        } else {
            self.disconnect();
        }
        self.transcript = Transcript::default();
        self.settings.active_project = Some(id);
        self.settings.cwd = cwd;
        self.prompt = self.drafts.remove(&id).unwrap_or_default();
        self.error = None;
        self.connect(&self.ui_context.clone());
        self.save();
    }

    fn resume(&mut self, recent: Recent) {
        if recent.project_id != self.settings.active_project {
            return;
        }
        if self.thread_id.as_deref() == Some(&recent.id) {
            if self.backend.is_none() {
                self.connect(&self.ui_context.clone());
            }
            return;
        }
        if let Some(index) = self
            .sessions
            .iter()
            .position(|s| s.thread_id.as_deref() == Some(&recent.id))
        {
            self.activate_session(index);
            return;
        }
        if self.backend.is_none() || (!self.ready && self.thread_id.is_none()) {
            self.resume_after_connect = Some(recent);
            self.connect(&self.ui_context.clone());
            return;
        }
        // A fresh backend must finish its handshake before resuming a disk-only chat.
        if self.busy || self.loading || !self.approvals.is_empty() {
            self.park_conversation();
            self.connect(&self.ui_context.clone());
            self.resume_after_connect = Some(recent);
            return;
        }
        self.new_chat();
        self.settings.model = recent.model.clone();
        self.settings.effort = recent.effort.clone();
        self.active_efficiency = recent.efficiency.clone();
        let mut params = json!({"threadId":recent.id,"excludeTurns":true,
            "cwd":self.settings.cwd});
        if !recent.model.is_empty() {
            params["model"] = recent.model.clone().into();
        }
        self.project_instructions(&mut params);
        if self.prepare_thread(params, true) {
            self.loading = true;
            self.status = "이전 대화 불러오는 중…".into();
        }
    }

    fn compact(&mut self) {
        if !self.ready || self.busy || self.loading {
            return;
        }
        let Some(id) = self.thread_id.clone() else {
            return;
        };
        self.compact_due = false;
        if self.rpc(
            "thread/compact/start",
            json!({"threadId": id}),
            Pending::Compact,
        ) {
            let project = self
                .settings
                .active()
                .map(|p| p.name.as_str())
                .unwrap_or("");
            self.meter.begin(
                project,
                &self.settings.model,
                &self.effective_effort(),
                "context compaction",
            );
            self.compacting = true;
            self.busy = true;
            self.started = Some(Instant::now());
            self.status = "컨텍스트 압축 중…".into();
        }
    }

    fn move_chat(&mut self, id: &str, project_id: u64) {
        if self.busy
            || self.loading
            || self.sessions.iter().any(|s| {
                s.thread_id.as_deref() == Some(id)
                    && (s.busy || s.loading || !s.approvals.is_empty())
            })
        {
            return;
        }
        let Some(project) = self.settings.projects.iter().find(|p| p.id == project_id) else {
            return;
        };
        let cwd = project.cwd.clone();
        if let Some(recent) = self.settings.recent.iter_mut().find(|r| r.id == id) {
            recent.project_id = Some(project_id);
            recent.cwd = cwd;
        } else {
            return;
        }
        if self.thread_id.as_deref() == Some(id) {
            self.new_chat();
        }
        self.sessions.retain(|s| s.thread_id.as_deref() != Some(id));
        self.save();
        self.status = "대화를 이동했습니다. 대상 프로젝트에서 이어가세요.".into();
    }

    fn chat_context_menu(&self, ui: &mut egui::Ui, recent: &Recent) -> Option<ChatAction> {
        let lang = self.settings.language;
        let mut action = None;
        let running = self.sessions.iter().any(|s| {
            s.thread_id.as_deref() == Some(&recent.id)
                && (s.busy || s.loading || !s.approvals.is_empty())
        });
        ui.add_enabled_ui(!self.busy && !self.loading && !running, |ui| {
            ui.menu_button(lang.text("다른 프로젝트로 이동"), |ui| {
                ui.small(lang.text("이후 작업에는 대상 폴더와 지침이 적용됩니다."));
                for project in &self.settings.projects {
                    if Some(project.id) != recent.project_id && ui.button(&project.name).clicked() {
                        action = Some(ChatAction::Move(recent.id.clone(), project.id));
                        ui.close();
                    }
                }
            });
            if ui
                .add_enabled(self.ready, egui::Button::new(lang.text("대화 삭제…")))
                .clicked()
            {
                action = Some(ChatAction::Delete(recent.clone()));
                ui.close();
            }
            if self.thread_id.as_deref() == Some(&recent.id)
                && ui
                    .add_enabled(
                        self.ready,
                        egui::Button::new(lang.text("지금 컨텍스트 압축")),
                    )
                    .clicked()
            {
                action = Some(ChatAction::Compact);
                ui.close();
            }
        });
        if ui.button(lang.text("대화 ID 복사")).clicked() {
            ui.ctx().copy_text(recent.id.clone());
            ui.close();
        }
        action
    }

    fn apply_chat_action(&mut self, action: ChatAction) {
        match action {
            ChatAction::Move(id, project) => self.move_chat(&id, project),
            ChatAction::Delete(recent) => self.delete_chat = Some(recent),
            ChatAction::Compact => self.compact(),
        }
    }

    fn delete_chat_ui(&mut self, ctx: &egui::Context) {
        let Some(recent) = self.delete_chat.clone() else {
            return;
        };
        let lang = self.settings.language;
        let response = egui::Modal::new(egui::Id::new("delete_chat")).show(ctx, |ui| {
            ui.set_max_width((ctx.content_rect().width() - 64.0).min(360.0));
            ui.heading(lang.text("대화 삭제"));
            ui.label(&recent.title);
            ui.label(lang.text("원본 대화와 하위 대화를 영구 삭제합니다. 되돌릴 수 없습니다. 작업 파일은 유지됩니다."));
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.loading, egui::Button::new(lang.text("취소"))).clicked() { self.delete_chat = None; }
                if ui.add_enabled(self.ready && !self.loading && !self.busy, egui::Button::new(lang.text("영구 삭제"))).clicked() {
                    self.loading = self.rpc("thread/delete", json!({"threadId":recent.id}), Pending::Delete(recent.id.clone()));
                }
            });
        });
        if response.should_close() && !self.loading {
            self.delete_chat = None;
        }
    }

    fn chat_options_ui(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        let instructions = self
            .settings
            .active()
            .map(|p| p.instructions.clone())
            .unwrap_or_default();
        let recommended = crate::recommendation::model(&self.models, &instructions)
            .map(|m| string(m, "model").to_owned());
        let old = (self.settings.model.clone(), self.settings.effort.clone());
        ui.horizontal_wrapped(|ui| {
            if self.thread_id.is_none() {
                ui.label(lang.text("모델"));
                ui.add_enabled_ui(!self.loading && !self.busy, |ui| {
                    egui::ComboBox::from_id_salt("chat_model")
                        .width(145.0)
                        .truncate()
                        .selected_text(if self.settings.model.is_empty() {
                            lang.text("Codex 기본값")
                        } else {
                            &self.settings.model
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.settings.model,
                                String::new(),
                                lang.text("Codex 기본값"),
                            );
                            for model in &self.models {
                                let name = string(model, "model");
                                ui.horizontal(|ui| {
                                    ui.selectable_value(
                                        &mut self.settings.model,
                                        name.to_owned(),
                                        name,
                                    )
                                    .on_hover_text(string(model, "description"));
                                    if recommended.as_deref() == Some(name) {
                                        ui.small(lang.text("권장")).on_hover_text(
                                            lang.text("권장: 프로젝트 지침 기반의 로컬 추천"),
                                        );
                                    }
                                });
                            }
                        });
                });
            }
            if self.thread_id.is_some() {
                ui.small(format!("{}: {}", lang.text("모델"), self.settings.model));
            }
            if old.0 != self.settings.model {
                self.settings.effort.clear();
            }
            // Keep effort and auto-accept together when the model row is too full.
            if ui.max_rect().right() - ui.cursor().left() < 260.0 {
                ui.end_row();
            }
            ui.horizontal(|ui| {
                let model = self
                    .models
                    .iter()
                    .find(|m| string(m, "model") == self.settings.model)
                    .or_else(|| {
                        self.models
                            .iter()
                            .find(|m| self.settings.model.is_empty() && m["isDefault"] == true)
                    });
                let recommended_effort = model
                    .filter(|_| !instructions.trim().is_empty())
                    .map(|m| crate::recommendation::supported_effort(m, &instructions))
                    .unwrap_or("");
                ui.label(lang.text("노력"))
                    .on_hover_text(lang.text("변경한 노력은 다음 전송부터 적용됩니다."));
                egui::ComboBox::from_id_salt("chat_effort")
                    .width(80.0)
                    .selected_text(if self.settings.effort.is_empty() {
                        lang.text("기본값")
                    } else {
                        &self.settings.effort
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.settings.effort,
                            String::new(),
                            lang.text("기본값"),
                        );
                        if let Some(efforts) =
                            model.and_then(|m| m["supportedReasoningEfforts"].as_array())
                        {
                            for effort in efforts {
                                let name = string(effort, "reasoningEffort");
                                ui.horizontal(|ui| {
                                    ui.selectable_value(
                                        &mut self.settings.effort,
                                        name.to_owned(),
                                        name,
                                    );
                                    if name == recommended_effort {
                                        ui.small(lang.text("권장")).on_hover_text(
                                            lang.text("권장: 프로젝트 지침 기반의 로컬 추천"),
                                        );
                                    }
                                });
                            }
                        }
                    })
                    .response
                    .on_hover_text(lang.text("변경한 노력은 다음 전송부터 적용됩니다."));
                self.auto_accept_ui(ui);
            });
        });
        if old != (self.settings.model.clone(), self.settings.effort.clone()) {
            if let Some(recent) = self
                .settings
                .recent
                .iter_mut()
                .find(|r| Some(&r.id) == self.thread_id.as_ref())
            {
                recent.effort = self.settings.effort.clone();
            }
            self.save();
        }
    }

    fn projects_ui(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        let can_change = !self.busy && !self.loading;
        ui.horizontal(|ui| {
            ui.strong(lang.text("프로젝트"));
            if ui.button(lang.text("추가")).clicked() {
                self.project_error = None;
                self.project_editor = Some(Project {
                    id: 0,
                    name: String::new(),
                    cwd: self.settings.cwd.clone(),
                    instructions: String::new(),
                    efficiency: Default::default(),
                });
            }
        });
        let mut selected = None;
        egui::ScrollArea::vertical()
            .id_salt("projects_list")
            .max_height(180.0)
            .show(ui, |ui| {
                for project in &self.settings.projects {
                    if ui
                        .add_enabled(
                            true,
                            egui::Button::new(&project.name)
                                .min_size(egui::vec2(ui.available_width(), 28.0))
                                .selected(Some(project.id) == self.settings.active_project)
                                .frame(Some(project.id) == self.settings.active_project)
                                .wrap_mode(egui::TextWrapMode::Truncate),
                        )
                        .on_hover_text(&project.cwd)
                        .clicked()
                    {
                        selected = Some(project.id);
                    }
                }
            });
        if let Some(id) = selected {
            self.switch_project(id);
        }
        if ui
            .add_enabled(
                can_change,
                egui::Button::new(lang.text("프로젝트 편집")).frame(false),
            )
            .clicked()
        {
            self.project_error = None;
            self.project_editor = self.settings.active().cloned();
        }
        ui.separator();
        if ui
            .add_enabled(
                self.ready || self.backend.is_some(),
                egui::Button::new(lang.text("새 대화")),
            )
            .clicked()
        {
            self.new_chat();
        }
        if !self.ready {
            ui.small(lang.text("연결 후 대화를 열 수 있습니다."));
        }
        ui.small(lang.text("최근 대화 · 최대 20개"));
        let mut chosen = None;
        let mut action = None;
        egui::ScrollArea::vertical()
            .id_salt("project_chats")
            .show(ui, |ui| {
                if !self
                    .settings
                    .recent
                    .iter()
                    .any(|r| r.project_id == self.settings.active_project)
                {
                    ui.weak(lang.text("최근 대화 없음"));
                }
                for recent in self
                    .settings
                    .recent
                    .iter()
                    .filter(|r| r.project_id == self.settings.active_project)
                {
                    let response = ui
                        .add_enabled(
                            self.ready || self.backend.is_some(),
                            egui::Button::new(&recent.title)
                                .min_size(egui::vec2(ui.available_width(), 28.0))
                                .selected(self.thread_id.as_deref() == Some(&recent.id))
                                .frame(self.thread_id.as_deref() == Some(&recent.id))
                                .wrap_mode(egui::TextWrapMode::Truncate),
                        )
                        .on_hover_text(&recent.title);
                    response.context_menu(|ui| {
                        action = self.chat_context_menu(ui, recent);
                    });
                    if response.clicked() {
                        chosen = Some(recent.clone());
                    }
                }
            });
        if let Some(action) = action {
            self.apply_chat_action(action);
        }
        if let Some(recent) = chosen {
            self.resume(recent);
        }
    }

    fn project_editor_ui(&mut self, ctx: &egui::Context) {
        let lang = self.settings.language;
        let Some(mut editor) = self.project_editor.take() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        egui::Window::new(if editor.id == 0 {
            lang.text("프로젝트 추가")
        } else {
            lang.text("프로젝트 편집")
        })
        .id(egui::Id::new("project_editor"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_size(egui::vec2(
            (ctx.content_rect().width() - 40.0).min(420.0),
            (ctx.content_rect().height() - 80.0).min(350.0),
        ))
        .vscroll(true)
        .show(ctx, |ui| {
            ui.label(lang.text("이름"));
            ui.add(
                egui::TextEdit::singleline(&mut editor.name)
                    .char_limit(120)
                    .desired_width(f32::INFINITY),
            );
            ui.label(lang.text("작업 폴더"));
            ui.add(
                egui::TextEdit::singleline(&mut editor.cwd)
                    .char_limit(4096)
                    .desired_width(f32::INFINITY),
            );
            ui.label(lang.text("프로젝트 지침 · 선택 사항"));
            ui.add(
                egui::TextEdit::multiline(&mut editor.instructions)
                    .char_limit(16 * 1024)
                    .desired_rows(5)
                    .desired_width(f32::INFINITY)
                    .hint_text(lang.text("이 프로젝트에서 공통으로 지킬 작업 방식")),
            )
            .on_hover_text(lang.text("지침은 이 프로젝트의 새 대화·이어쓰기에 적용됩니다."));

            if let Some(error) = &self.project_error {
                ui.colored_label(ui.visuals().error_fg_color, lang.message(error));
            }
            save = ui
                .add_enabled(
                    editor.id == 0 || (!self.busy && !self.loading),
                    egui::Button::new(lang.text("저장")),
                )
                .on_hover_text(
                    lang.text("현재 작업을 유지합니다. 새 프로젝트는 작업이 끝난 뒤 선택하세요."),
                )
                .clicked();
        });
        if save {
            let result = (|| -> Result<(), String> {
                if editor.name.trim().is_empty()
                    || editor.name.len() > 120
                    || editor.instructions.len() > 16 * 1024
                {
                    return Err("이름은 1~120바이트, 지침은 16 KiB 이하로 입력하세요.".into());
                }
                let cwd = std::fs::canonicalize(&editor.cwd)
                    .map_err(|_| "존재하는 작업 폴더를 입력하세요.")?;
                if !cwd.is_dir() {
                    return Err("작업 폴더를 선택하세요.".into());
                }
                editor.cwd = cwd.display().to_string();
                if editor.id == 0 {
                    let id = self.settings.add_project(
                        &editor.name,
                        &editor.cwd,
                        &editor.instructions,
                    )?;
                    if self.busy || self.loading {
                        self.save();
                    } else {
                        self.switch_project(id);
                    }
                } else {
                    if self.busy || self.loading {
                        return Err("작업이 끝난 뒤 프로젝트를 편집하세요.".into());
                    }
                    let Some(existing) = self
                        .settings
                        .projects
                        .iter_mut()
                        .find(|p| p.id == editor.id)
                    else {
                        return Err("프로젝트를 찾을 수 없습니다.".into());
                    };
                    *existing = editor.clone();
                    if self.settings.active_project == Some(editor.id) {
                        self.disconnect();
                        self.settings.cwd = editor.cwd.clone();
                        self.transcript = Transcript::default();
                        self.connect(&self.ui_context.clone());
                    }
                    self.save();
                }
                Ok(())
            })();
            match result {
                Ok(()) => open = false,
                Err(error) => self.project_error = Some(error),
            }
        }
        if open && !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.project_editor = Some(editor);
        }
    }

    fn process(&mut self, message: Value) {
        if message.get("method").is_some() {
            if message.get("id").is_some() {
                self.server_request(message);
            } else {
                self.notification(message);
            }
            return;
        }
        let Some(id) = message["id"].as_u64() else {
            return;
        };
        let Some(pending) = self.pending.remove(&id) else {
            return;
        };
        if let Some(error) = message.get("error") {
            if matches!(pending, Pending::Skills | Pending::SkillSaved) {
                self.global.message =
                    format!("스킬 설정 요청 실패: {}", clipped(&error.to_string(), 2000));
            }
            self.error = Some(clipped(&error.to_string(), 8000));
            match pending {
                Pending::Initialize => self.disconnect(),
                Pending::Delete(_) => self.loading = false,
                Pending::Compact => {
                    self.finish_usage("failed");
                    self.compacting = false;
                    self.busy = false;
                    self.compact_due = false;
                }
                Pending::Start
                | Pending::Resume
                | Pending::History
                | Pending::PrepareThread { .. } => {
                    self.loading = false;
                    self.queued_prompt = None;
                }
                Pending::Turn => {
                    self.finish_usage("failed");
                    self.busy = false;
                    self.turn_id = None;
                    if let Some(id) = self.submitted_message_id.take() {
                        self.transcript.messages.retain(|m| m.id != id);
                    }
                    if self.prompt.is_empty() {
                        self.prompt = self.submitted_prompt.clone();
                    }
                }
                Pending::Steer { local_id, prompt } => {
                    self.transcript.messages.retain(|m| m.id != local_id);
                    if self.prompt.is_empty() {
                        self.prompt = prompt;
                    } else {
                        self.note("추가 메시지 전송 실패 · 다시 보내세요", &prompt);
                    }
                }
                _ => {}
            }
            self.status = "요청 실패 · 내용을 확인하세요".into();
            return;
        }
        let result = &message["result"];
        match pending {
            Pending::Compact => {} // Completion arrives through turn/completed.
            Pending::Delete(id) => {
                self.loading = false;
                if self.thread_id.as_deref() == Some(&id) {
                    self.new_chat();
                }
                self.settings.recent.retain(|r| r.id != id);
                self.discarded_threads.push(id);
                self.delete_chat = None;
                self.save();
                self.status = "대화를 삭제했습니다.".into();
            }
            Pending::PrepareThread { params, resume } => {
                if !self.start_thread(params, resume, &result["config"]) {
                    self.loading = false;
                    self.queued_prompt = None;
                }
            }
            Pending::LocalSkills => {
                self.local_skills = result["data"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|d| d["skills"].as_array().into_iter().flatten())
                    .take(2000)
                    .cloned()
                    .collect();
            }
            Pending::Skills => {
                self.global.skills = result["data"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|d| d["skills"].as_array().into_iter().flatten())
                    .filter(|s| s["scope"].as_str() != Some("repo"))
                    .take(2000)
                    .cloned()
                    .collect();
                self.global.skills_loaded = true;
                self.global.message = if self.global.skills.is_empty() {
                    "전역 스킬이 없습니다.".into()
                } else {
                    String::new()
                };
            }
            Pending::SkillSaved => {
                self.global.refresh_config(&self.ui_context);
                self.global.changed = true;
                self.global.message = "스킬 설정을 저장했습니다. 다시 연결하면 적용됩니다.".into();
                self.rpc(
                    "skills/list",
                    json!({"cwds":[self.settings.cwd],"forceReload":true}),
                    Pending::Skills,
                );
            }
            Pending::Initialize => {
                self.send(json!({"method":"initialized","params":{}}));
                self.ready = true;
                self.loading = false;
                self.status = "연결됨 · 대기 중".into();
                if !self.background_processing {
                    self.show_settings = false;
                }
                self.rpc(
                    "account/read",
                    json!({"refreshToken":false}),
                    Pending::Account,
                );
                self.rpc("model/list", json!({"limit":50}), Pending::Models);
            }
            Pending::Account => {
                let account = &result["account"];
                self.account = if account.is_null() {
                    if result["requiresOpenaiAuth"].as_bool() == Some(true) {
                        "로그인 필요".into()
                    } else {
                        "로컬 설정 사용".into()
                    }
                } else {
                    format!(
                        "{} {}",
                        string(account, "type"),
                        string(account, "planType")
                    )
                };
            }
            Pending::Models => {
                if let Some(data) = result["data"].as_array() {
                    self.models = data.clone();
                }
            }
            Pending::Start | Pending::Resume => {
                self.loading = false;
                let thread = string(&result["thread"], "id").to_owned();
                if thread.is_empty() {
                    self.error = Some("Codex가 대화 ID를 반환하지 않았습니다.".into());
                    self.queued_prompt = None;
                    return;
                }
                self.meter
                    .attach(&thread, matches!(pending, Pending::Start));
                self.thread_id = Some(thread.clone());
                if let Some(model) = result["model"].as_str() {
                    self.settings.model = model.to_owned();
                }
                if matches!(pending, Pending::Resume) {
                    self.loading = self.rpc("thread/turns/list",json!({"threadId":thread,"limit":20,"itemsView":"full","sortDirection":"desc"}),Pending::History);
                } else {
                    self.transcript = Transcript::default();
                    if let Some(prompt) = self.queued_prompt.take() {
                        self.start_turn(prompt);
                    }
                }
            }
            Pending::History => {
                self.loading = false;
                if let Some(turns) = result["data"].as_array() {
                    for turn in turns.iter().rev() {
                        if let Some(items) = turn["items"].as_array() {
                            for item in items {
                                self.transcript.item(item);
                            }
                        }
                    }
                }
                if !result["nextCursor"].is_null() {
                    self.transcript.trimmed = true;
                }
                self.status = "대화를 이어갈 수 있습니다".into();
            }
            Pending::Turn => {
                // turn/completed may precede the response; never resurrect a finished turn.
                if self.busy
                    && let Some(id) = result["turn"]["id"].as_str()
                {
                    self.turn_id = Some(id.into());
                    self.meter.turn(id);
                }
            }
            Pending::Interrupt => self.status = "중단 요청됨…".into(),
            Pending::Steer { .. } => {} // Same turn: preserve its timer, usage and approvals.
            Pending::Login => self.auth_url = result["authUrl"].as_str().map(str::to_owned),
            Pending::Ignore => {}
        }
    }

    fn notification(&mut self, message: Value) {
        let method = string(&message, "method");
        let p = &message["params"];
        if let Some(thread) = p["threadId"].as_str()
            && self.thread_id.as_deref() != Some(thread)
        {
            return;
        }
        match method {
            "thread/tokenUsage/updated" => {
                if !self.compacting {
                    let usage = &p["tokenUsage"];
                    self.compact_due = usage["modelContextWindow"].as_u64().is_some_and(|limit| {
                        limit > 0
                            && usage["last"]["totalTokens"].as_u64().unwrap_or(0)
                                >= limit.saturating_mul(4) / 5
                    });
                }
                if let Some(record) = self.meter.update(p) {
                    self.persist_usage(record);
                }
                // Usage can arrive after turn/completed; do not miss that ordering.
                if self.compact_due && !self.busy && !self.loading {
                    self.compact();
                }
            }
            "item/started" | "item/completed" => {
                if string(&p["item"], "type") == "contextCompaction" {
                    self.compact_due = false;
                    self.status = if method == "item/started" {
                        "컨텍스트 압축 중…"
                    } else {
                        "컨텍스트 압축 완료"
                    }
                    .into();
                }
                self.meter.tool(&p["item"]);
                self.transcript.item(&p["item"]);
            }
            "item/agentMessage/delta" => self.transcript.put(
                string(p, "itemId"),
                "CODEX",
                string(p, "delta"),
                false,
                true,
            ),
            "item/commandExecution/outputDelta" => {
                // Final command item contains its bounded output; avoid rendering every byte.
            }
            "turn/started" => {
                self.turn_id = p["turn"]["id"].as_str().map(str::to_owned);
                if let Some(id) = &self.turn_id {
                    self.meter.turn(id);
                }
                self.busy = true;
            }
            "turn/completed" => {
                let turn = string(&p["turn"], "id");
                if !turn.is_empty() {
                    let key = format!("{}:{turn}", self.thread_id.as_deref().unwrap_or(""));
                    if self.completed_turns.contains(&key) {
                        return;
                    }
                    self.completed_turns.push_back(key);
                    if self.completed_turns.len() > 64 {
                        self.completed_turns.pop_front();
                    }
                }
                if self.compacting {
                    if let Some(id) = p["turn"]["id"].as_str() {
                        self.meter.turn(id);
                    }
                    self.finish_usage(string(&p["turn"], "status"));
                    self.compacting = false;
                    self.busy = false;
                    self.compact_due = false;
                    self.turn_id = None;
                    self.started = None;
                    self.status = if string(&p["turn"], "status") == "completed" {
                        "컨텍스트 압축 완료"
                    } else {
                        "컨텍스트 압축 실패"
                    }
                    .into();
                    if string(&p["turn"], "status") == "failed" {
                        self.error = Some(p["turn"]["error"].to_string());
                    }
                    return;
                }
                let was_running = self.busy || self.started.is_some();
                if let Some(id) = p["turn"]["id"].as_str() {
                    self.meter.turn(id);
                }
                self.finish_usage(string(&p["turn"], "status"));
                self.busy = false;
                self.turn_id = None;
                self.approvals.clear();
                let status = string(&p["turn"], "status");
                let elapsed = self
                    .started
                    .take()
                    .map(|i| i.elapsed().as_secs())
                    .unwrap_or(0);
                self.status = match status {
                    "completed" => format!("완료 · {elapsed}초"),
                    "interrupted" => "작업 중단됨".into(),
                    _ => {
                        self.error = Some(clipped(&p["turn"]["error"].to_string(), 8000));
                        "작업 실패".into()
                    }
                };
                if was_running
                    && self.settings.notifications
                    && matches!(status, "completed" | "failed")
                {
                    let lang = self.settings.language;
                    let project = self
                        .settings
                        .active()
                        .map(|p| p.name.as_str())
                        .unwrap_or("Codex Mini");
                    self.desktop.notify(
                        lang.text(if status == "completed" {
                            "Codex Mini · 작업 완료"
                        } else {
                            "Codex Mini · 작업 실패"
                        }),
                        &lang.format(
                            if status == "completed" {
                                "{project} · 작업이 완료되었습니다."
                            } else {
                                "{project} · 작업을 완료하지 못했습니다. 앱에서 확인하세요."
                            },
                            &[&project],
                        ),
                    );
                }
                if status == "completed" && self.compact_due {
                    self.compact();
                }
            }
            "serverRequest/resolved" => self.approvals.retain(|a| a.id != p["requestId"]),
            "error" => {
                self.error = Some(clipped(&p["error"].to_string(), 8000));
            }
            "account/updated" | "account/login/completed" => {
                self.auth_url = None;
                self.rpc(
                    "account/read",
                    json!({"refreshToken":false}),
                    Pending::Account,
                );
                if method == "account/login/completed" {
                    if p["success"].as_bool() == Some(false) {
                        self.error = Some(p["error"].to_string());
                    } else {
                        self.rpc("model/list", json!({"limit":50}), Pending::Models);
                    }
                }
            }
            _ => {}
        }
    }

    fn server_request(&mut self, message: Value) {
        let method = string(&message, "method").to_owned();
        let p = message["params"].clone();
        let id = message["id"].clone();
        let supported = matches!(
            method.as_str(),
            "item/commandExecution/requestApproval"
                | "item/fileChange/requestApproval"
                | "item/permissions/requestApproval"
                | "item/tool/requestUserInput"
        );
        if !supported || self.approvals.len() >= 16 {
            if method == "mcpServer/elicitation/request" {
                self.send(json!({"id":id,"result":{"action":"decline","content":null}}));
            } else {
                self.send(json!({"id":id,"error":{"code":-32601,"message":"This compact client does not support this request"}}));
            }
            self.note("지원하지 않는 요청", &method);
            return;
        }
        let previous = self
            .transcript
            .messages
            .iter()
            .find(|m| m.id == string(&p, "itemId"))
            .map(|m| m.body.clone())
            .unwrap_or_default();
        let preview_language = self.settings.language;
        let (preview, preview_complete) = approval_preview(&p, &previous, preview_language);
        let is_question = method == "item/tool/requestUserInput";
        let request_id = id.clone();
        self.approvals.push_back(Approval {
            id,
            method,
            params: p,
            preview,
            preview_complete,
            previous,
            preview_language,
            answers: HashMap::new(),
        });
        self.accept_pending_automatically();
        if self.settings.notifications
            && self.approvals.iter().any(|a| a.id == request_id)
            && (is_question || self.approvals.len() == 1)
        {
            let lang = self.settings.language;
            self.desktop.notify(
                lang.text(if is_question {
                    "Codex Mini · 답변 필요"
                } else {
                    "Codex Mini · 확인 필요"
                }),
                lang.text(if is_question {
                    "추가 질문이 도착했습니다. 앱에서 질문을 확인하고 답변하세요."
                } else {
                    "작업을 계속하려면 앱에서 요청 내용을 확인하세요."
                }),
            );
        }
    }

    fn auto_accept_ui(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        if ui.add(egui::Button::new(lang.text(if self.auto_accept {
            "자동수락: 켜짐"
        } else {
            "자동수락: 꺼짐"
        })).selected(self.auto_accept))
            .on_hover_text(lang.text("이번 연결에서 명령·파일 변경·권한 요청을 자동수락합니다. 직접 답변하는 질문은 제외됩니다. 연결 종료 시 꺼집니다."))
            .clicked()
        {
            self.auto_accept = !self.auto_accept;
            self.accept_pending_automatically();
        }
    }

    fn accept_pending_automatically(&mut self) {
        if !self.auto_accept {
            return;
        }
        let mut index = 0;
        while index < self.approvals.len() {
            let a = &self.approvals[index];
            let result = automatic_approval(a);
            if let Some(result) = result {
                let id = a.id.clone();
                if !self.send(json!({"id":id,"result":result})) {
                    break; // Keep the request available for a manual retry.
                }
                let a = self.approvals.remove(index).unwrap();
                self.note("자동수락", &a.preview);
            } else {
                index += 1;
            }
        }
    }

    fn note(&mut self, title: &str, body: &str) {
        self.serial += 1;
        self.transcript
            .put(&format!("note-{}", self.serial), title, body, true, false);
    }

    fn approval_ui(&mut self, ctx: &egui::Context) {
        let lang = self.settings.language;
        let mut response = None;
        if let Some(a) = self.approvals.front_mut() {
            if a.preview_language != lang {
                (a.preview, a.preview_complete) = approval_preview(&a.params, &a.previous, lang);
                a.preview_language = lang;
            }
            egui::Window::new(lang.text("확인이 필요합니다")).id(egui::Id::new("approval"))
                .collapsible(false)
                .resizable(true)
                .default_width(520.0_f32.min(ctx.content_rect().width() - 24.0))
                .max_width((ctx.content_rect().width() - 24.0).max(0.0))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        // Keep the warning and decisions visible in small or zoomed windows.
                        .max_height((ctx.content_rect().height() - 180.0).clamp(32.0, 300.0))
                        .min_scrolled_height(0.0)
                        .show(ui, |ui| {
                            if a.method == "item/tool/requestUserInput" {
                                if let Some(questions) = a.params["questions"].as_array() {
                                    for q in questions {
                                        ui.strong(string(q, "question"));
                                        let answer =
                                            a.answers.entry(string(q, "id").into()).or_default();
                                        if let Some(options) = q["options"].as_array() {
                                            for option in options {
                                                let label = string(option, "label");
                                                ui.radio_value(answer, label.to_owned(), label)
                                                    .on_hover_text(string(option, "description"));
                                            }
                                        }
                                        ui.add(
                                            egui::TextEdit::singleline(answer)
                                                .password(q["isSecret"].as_bool() == Some(true))
                                                .hint_text(lang.text("답변 직접 입력")),
                                        );
                                        ui.separator();
                                    }
                                }
                            } else {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&a.preview).monospace().size(10.0),
                                    )
                                    .wrap()
                                    .selectable(true),
                                );
                            }
                        });
                    ui.separator();
                    if a.method == "item/permissions/requestApproval" && !a.params["permissions"].is_object() {
                        ui.colored_label(ui.visuals().error_fg_color, lang.text("요청한 권한 정보가 없어 허용할 수 없습니다."));
                    } else if !a.preview_complete && a.method != "item/tool/requestUserInput" {
                        ui.colored_label(ui.visuals().warn_fg_color, lang.text("미리보기가 비어 있거나 일부 내용이 생략되었습니다. 이 요청은 자동수락하지 않습니다. 직접 허용하거나 거절하세요."));
                    }
                    ui.horizontal_wrapped(|ui| {
                        if a.method == "item/tool/requestUserInput" {
                            let complete = a.params["questions"].as_array().is_some_and(|qs| {
                                qs.iter().all(|q| {
                                    a.answers
                                        .get(string(q, "id"))
                                        .is_some_and(|s| !s.trim().is_empty())
                                })
                            });
                            if ui
                                .add_enabled(complete, egui::Button::new(lang.text("답변 보내기")))
                                .clicked()
                            {
                                let answers: serde_json::Map<String, Value> = a
                                    .answers
                                    .iter()
                                    .map(|(k, v)| (k.clone(), json!({"answers":[v]})))
                                    .collect();
                                response = Some(json!({"answers":answers}));
                            }
                        } else if a.method == "item/permissions/requestApproval" {
                            if ui.add_enabled(a.params["permissions"].is_object(), egui::Button::new(lang.text("요청한 권한 허용 · 이번 작업만"))).clicked() {
                                response = Some(
                                    json!({"permissions":a.params["permissions"],"scope":"turn"}),
                                );
                            }
                            if ui.button(lang.text("거절")).clicked() {
                                response = Some(json!({"permissions":{},"scope":"turn"}));
                            }
                        } else {
                            let decisions = a.params["availableDecisions"].as_array();
                            let can_accept = decisions
                                .is_none_or(|ds| ds.iter().any(|v| v.as_str() == Some("accept")));
                            if ui
                                .add_enabled(can_accept, egui::Button::new(lang.text("이번 한 번 허용")))
                                .clicked()
                            {
                                response = Some(json!({"decision":"accept"}));
                            }
                            let can_decline = decisions
                                .is_none_or(|ds| ds.iter().any(|v| v.as_str() == Some("decline")));
                            if ui
                                .add_enabled(can_decline, egui::Button::new(lang.text("거절")))
                                .clicked()
                            {
                                response = Some(json!({"decision":"decline"}));
                            }
                        }

                    });
                });
        }
        if let Some(result) = response {
            let id = self.approvals.front().unwrap().id.clone();
            if self.send(json!({"id":id,"result":result})) {
                self.approvals.pop_front();
            }
        }
    }

    fn prepare_thread(&mut self, params: Value, resume: bool) -> bool {
        if self.active_efficiency.rtk || !self.active_efficiency.disabled_skills.is_empty() {
            self.rpc(
                "config/read",
                json!({"cwd":self.settings.cwd,"includeLayers":false}),
                Pending::PrepareThread { params, resume },
            )
        } else {
            self.start_thread(params, resume, &json!({}))
        }
    }

    fn start_thread(&mut self, mut params: Value, resume: bool, inherited: &Value) -> bool {
        let config = std::env::current_exe()
            .map_err(|e| e.to_string())
            .and_then(|exe| {
                self.active_efficiency
                    .config(&exe, self.rtk_path.as_deref(), inherited)
            });
        match config {
            Ok(mut config) => {
                let effort = self.effective_effort();
                if !effort.is_empty() {
                    config["model_reasoning_effort"] = effort.into();
                } else if let Some(fields) = config.as_object_mut() {
                    fields.remove("model_reasoning_effort");
                }
                if config.as_object().is_some_and(|c| !c.is_empty()) {
                    params["config"] = config;
                }
                self.rpc(
                    if resume {
                        "thread/resume"
                    } else {
                        "thread/start"
                    },
                    params,
                    if resume {
                        Pending::Resume
                    } else {
                        Pending::Start
                    },
                )
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    fn effective_effort(&self) -> String {
        let requested = if self.settings.effort.is_empty() {
            &self.active_efficiency.effort
        } else {
            &self.settings.effort
        };
        let model = self
            .models
            .iter()
            .find(|m| string(m, "model") == self.settings.model)
            .or_else(|| {
                self.models
                    .iter()
                    .find(|m| self.settings.model.is_empty() && m["isDefault"] == true)
            });
        if let Some(model) = model {
            let supported = model["supportedReasoningEfforts"].as_array();
            if requested.is_empty()
                || supported
                    .is_some_and(|xs| !xs.iter().any(|e| string(e, "reasoningEffort") == requested))
            {
                // Explicitly reset to the catalog default: omission can retain the previous turn's effort.
                return string(model, "defaultReasoningEffort").to_owned();
            }
        }
        requested.clone()
    }

    fn persist_usage(&mut self, record: crate::usage::Record) {
        if let Some(journal) = &self.journal
            && let Err(error) = journal.save(record)
        {
            self.usage_error = Some(error);
        }
    }

    fn finish_usage(&mut self, status: &str) {
        if let Some(record) = self.meter.finish(status) {
            self.persist_usage(record);
        }
    }

    fn usage_window(&mut self, ctx: &egui::Context) {
        let lang = self.settings.language;
        if !self.show_usage {
            return;
        }
        let mut open = true;
        egui::Window::new(lang.text("사용량·절약"))
            .id(egui::Id::new("usage"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::vec2(
                (ctx.content_rect().width() - 32.0).min(480.0),
                (ctx.content_rect().height() - 60.0).min(420.0),
            ))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .vscroll(true)
            .show(ctx, |ui| self.usage_ui(ui));
        if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_usage = false;
            self.save();
        }
    }

    fn usage_ui(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        ui.strong(lang.text("현재 작업")).on_hover_text(
            lang.text("캐시는 입력에, 추론은 출력에 포함됩니다. 절감률·요금 추정값이 아닙니다."),
        );
        if let Some(record) = self.meter.current() {
            if let Some(tokens) = &record.tokens {
                token_grid(ui, "current_tokens", tokens, lang);
            } else {
                ui.label(lang.text("서버 사용량 수신 대기 중"));
            }
            ui.label(lang.format(
                "{} · {} · 도구 {}회 · {}초",
                &[
                    &(record.model),
                    &(if record.effort.is_empty() {
                        lang.text("Codex 기본값")
                    } else {
                        &record.effort
                    }),
                    &(record.tools),
                    &(record.elapsed_secs),
                ],
            ));
            if record.partial {
                ui.weak(
                    lang.text("복원 후 관측분만 포함합니다. 첫 호출 이전 사용량은 제외됩니다."),
                );
            }
        } else {
            ui.label(lang.text("아직 측정한 작업이 없습니다."));
        }
        if let Some(error) = &self.usage_error {
            ui.colored_label(ui.visuals().error_fg_color, lang.message(error));
        }
        ui.separator();
        ui.strong(lang.text("이 프로젝트 · 다음 새 대화부터"))
            .on_hover_text(lang.text("기존 대화는 시작할 때 선택한 설정을 유지합니다."));
        let Some(project_id) = self.settings.active_project else {
            return;
        };
        let Some(mut options) = self.settings.active().map(|p| p.efficiency.clone()) else {
            return;
        };
        let previous = options.clone();
        ui.horizontal(|ui| {
            ui.label(lang.text("기본 추론")).on_hover_text(
                lang.text("채팅창에서 지정한 노력이 우선합니다. 모델별 지원 범위가 다릅니다."),
            );
            egui::ComboBox::from_id_salt("project_effort")
                .selected_text(if options.effort.is_empty() {
                    lang.text("Codex 기본값")
                } else {
                    &options.effort
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut options.effort,
                        String::new(),
                        lang.text("Codex 기본값"),
                    );
                    let efforts = self
                        .models
                        .iter()
                        .find(|m| string(m, "model") == self.settings.model)
                        .and_then(|m| m["supportedReasoningEfforts"].as_array());
                    if let Some(efforts) = efforts {
                        for effort in efforts {
                            let name = string(effort, "reasoningEffort");
                            ui.selectable_value(&mut options.effort, name.to_owned(), name);
                        }
                    } else {
                        for name in ["medium", "high", "xhigh"] {
                            ui.selectable_value(&mut options.effort, name.to_owned(), name);
                        }
                    }
                });
        });
        ui.add_enabled(
            self.rtk_path.is_some() || options.rtk,
            egui::Checkbox::new(&mut options.rtk, lang.text("Cargo 로그 필터 (RTK)")),
        )
        .on_hover_text(if self.rtk_path.is_some() {
            lang.text(
                "test / clippy / build / check만 필터링. 생략된 진단은 RTK 복구 힌트를 확인하세요.",
            )
        } else if cfg!(windows) {
            lang.text("RTK 자동 필터는 Linux·macOS의 Bash 명령에서 지원됩니다.")
        } else {
            lang.text("RTK 미설치 · scripts/install-rtk.sh 실행 후 앱을 다시 여세요.")
        });
        ui.collapsing(lang.text("스킬·MCP 제외"), |ui| {
            if ui
                .add_enabled(
                    self.ready,
                    egui::Button::new(lang.text("스킬 목록 불러오기")),
                )
                .on_hover_text(
                    lang.text("필요 없는 항목만 선택하세요. 공유 Codex 설정은 바뀌지 않습니다."),
                )
                .clicked()
            {
                self.rpc(
                    "skills/list",
                    json!({"cwds":[self.settings.cwd],"forceReload":true}),
                    Pending::LocalSkills,
                );
            }
            for skill in &self.local_skills {
                let path = string(skill, "path");
                let mut excluded = options.disabled_skills.iter().any(|p| p == path);
                let globally_enabled = skill["enabled"].as_bool().unwrap_or(true);
                if ui
                    .add_enabled(
                        globally_enabled,
                        egui::Checkbox::new(
                            &mut excluded,
                            lang.format(
                                "제외: {}{}",
                                &[
                                    &(string(skill, "name")),
                                    &(if globally_enabled {
                                        ""
                                    } else {
                                        lang.text(" (이미 꺼짐)")
                                    }),
                                ],
                            ),
                        ),
                    )
                    .on_hover_text(path)
                    .changed()
                {
                    options.disabled_skills.retain(|p| p != path);
                    if excluded {
                        options.disabled_skills.push(path.into());
                    }
                }
            }
            ui.label(lang.text("MCP 제외 이름 · 쉼표 구분"));
            ui.add(
                egui::TextEdit::singleline(&mut self.mcp_draft)
                    .desired_width(ui.available_width())
                    .hint_text("server_name, another_server"),
            );
            if ui.button(lang.text("MCP 제외 적용")).clicked() {
                let names: Vec<_> = self
                    .mcp_draft
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if names.iter().all(|n| {
                    n.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                }) {
                    options.disabled_mcp = names.into_iter().map(str::to_owned).collect();
                    options.disabled_mcp.sort();
                    options.disabled_mcp.dedup();
                    self.usage_error = None;
                } else {
                    self.usage_error =
                        Some("MCP 이름은 영문·숫자·밑줄·하이픈만 사용할 수 있습니다.".into());
                }
            }
            if ui.button(lang.text("제외 항목 초기화")).clicked() {
                options.disabled_skills.clear();
                options.disabled_mcp.clear();
                self.mcp_draft.clear();
            }
            ui.label(lang.format(
                "스킬 {}개 · MCP {}개 제외",
                &[
                    &(options.disabled_skills.len()),
                    &(options.disabled_mcp.len()),
                ],
            ));
        });
        if options != previous {
            if let Some(project) = self
                .settings
                .projects
                .iter_mut()
                .find(|p| p.id == project_id)
            {
                project.efficiency = options;
            }
            self.save();
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.strong(lang.format(
                "최근 작업 · {} / {}",
                &[&(self.meter.records.len()), &(crate::usage::KEEP)],
            ))
            .on_hover_text(
                lang.text("이 PC의 최근 200개 작업. 프롬프트·코드·응답 본문은 기록하지 않습니다."),
            );
            if ui.button(lang.text("JSON 복사")).clicked() {
                ui.ctx().copy_text(
                    serde_json::to_string_pretty(&self.meter.records).unwrap_or_default(),
                );
            }
        });
        for record in &self.meter.records {
            ui.push_id((&record.thread, &record.turn), |ui| {
                ui.collapsing(
                    lang.format(
                        "{} · {} · {} · {}초",
                        &[
                            &(record.project),
                            &lang.message(&record.profile),
                            &(record.status),
                            &(record.elapsed_secs),
                        ],
                    ),
                    |ui| {
                        if let Some(tokens) = &record.tokens {
                            token_grid(ui, "record_tokens", tokens, lang);
                        } else {
                            ui.label(lang.text("서버 사용량 없음"));
                        }
                        ui.label(
                            lang.format("{} · 도구 {}회", &[&(record.model), &(record.tools)]),
                        );
                        if record.partial {
                            ui.weak(lang.text("복원 후 관측분"));
                        }
                    },
                );
            });
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        let old_preferences = (lang, self.settings.theme);
        const LABEL_WIDTH: f32 = 72.0;
        const ROW_HEIGHT: f32 = 24.0;
        let field_width = (ui.available_width() - LABEL_WIDTH - 12.0).max(120.0);
        let label = |ui: &mut egui::Ui, text: &str| {
            ui.allocate_ui_with_layout(
                egui::vec2(LABEL_WIDTH, ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(text);
                },
            );
        };
        ui.spacing_mut().interact_size.y = ROW_HEIGHT;
        ui.small(lang.text("화면"));
        egui::Grid::new("appearance_fields")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                label(ui, lang.text("언어"));
                ui.horizontal(|ui| {
                    let width = (field_width - ui.spacing().item_spacing.x) / 2.0;
                    for (value, text) in
                        [(Language::Korean, "한국어"), (Language::English, "English")]
                    {
                        if ui
                            .add_sized(
                                [width, ROW_HEIGHT],
                                egui::Button::new(text).selected(self.settings.language == value),
                            )
                            .clicked()
                        {
                            self.settings.language = value;
                        }
                    }
                });
                ui.end_row();
                label(ui, lang.text("테마"));
                ui.horizontal(|ui| {
                    let width = (field_width - ui.spacing().item_spacing.x) / 2.0;
                    for (value, text) in [
                        (crate::preferences::Theme::Light, lang.text("라이트")),
                        (crate::preferences::Theme::Dark, lang.text("다크")),
                    ] {
                        if ui
                            .add_sized(
                                [width, ROW_HEIGHT],
                                egui::Button::new(text).selected(self.settings.theme == value),
                            )
                            .clicked()
                        {
                            self.settings.theme = value;
                        }
                    }
                });
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.small("Codex");
        egui::Grid::new("settings_fields")
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                label(ui, lang.text("작업 폴더"));
                ui.add_enabled_ui(false, |ui| {
                    ui.add_sized(
                        [field_width, ROW_HEIGHT],
                        egui::TextEdit::singleline(&mut self.settings.cwd),
                    );
                });
                ui.end_row();
            });
        if old_preferences != (self.settings.language, self.settings.theme) {
            self.settings.theme.apply(ui.ctx());
            self.save();
            ui.ctx().request_repaint();
        }
        if ui
            .checkbox(
                &mut self.settings.notifications,
                lang.text("작업 완료 알림"),
            )
            .on_hover_text(lang.text("작업 완료와 추가 질문을 시스템 알림으로 보냅니다."))
            .changed()
        {
            self.save();
        }
        if self.desktop.last_error.is_some() {
            ui.colored_label(ui.visuals().warn_fg_color, lang.text("시스템 알림·트레이 상태를 확인하세요. 운영체제 알림 설정과 앱 진단을 확인할 수 있습니다."));
        }
    }

    fn prompt_ui(&mut self, ui: &mut egui::Ui, compact: bool) -> bool {
        let id = egui::Id::new("chat_prompt");
        let focused = ui.memory(|m| m.has_focus(id));
        let mut ime_in_frame = self.prompt_composing;
        let mut committed_in_frame = false;
        if focused {
            ui.input(|i| {
                for event in &i.events {
                    if let egui::Event::Ime(event) = event {
                        match event {
                            egui::ImeEvent::Preedit { text, .. } => {
                                ime_in_frame = true;
                                self.prompt_composing = !text.is_empty();
                            }
                            egui::ImeEvent::Commit(_) => {
                                committed_in_frame = true;
                                self.prompt_composing = false;
                            }
                            _ => {}
                        }
                    }
                }
            });
        } else {
            self.prompt_composing = false;
        }
        let mut send = false;
        // A Korean IME can commit its last syllable and forward Enter in the
        // same frame. TextEdit applies that commit before the caller submits.
        if focused && !self.prompt_composing && (!ime_in_frame || committed_in_frame) {
            ui.input_mut(|i| {
                i.events.retain(|event| {
                    if matches!(event, egui::Event::Key {
                    key: egui::Key::Enter, pressed: true, repeat: false, modifiers, ..
                } if *modifiers == egui::Modifiers::NONE)
                    {
                        send = true;
                        false
                    } else {
                        true
                    }
                })
            });
        }
        egui::ScrollArea::vertical()
            .id_salt("prompt_scroll")
            .max_height(if compact { 48.0 } else { 112.0 })
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.prompt)
                        .id(id)
                        .return_key(egui::KeyboardShortcut::new(
                            egui::Modifiers::SHIFT,
                            egui::Key::Enter,
                        ))
                        .char_limit(32 * 1024)
                        .desired_rows(if compact { 1 } else { 2 })
                        .desired_width(f32::INFINITY)
                        .frame(egui::Frame::NONE)
                        .hint_text(self.settings.language.text("메시지 입력")),
                )
                .on_hover_text(
                    self.settings
                        .language
                        .text("Enter 전송 · Shift+Enter 줄바꿈"),
                );
            });
        send
    }

    fn show_main_window(&mut self, ctx: &egui::Context) {
        self.hidden = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }

    fn hide_main_window(&mut self, ctx: &egui::Context) {
        self.show_exit_confirmation = false;
        self.hidden = self.desktop.has_tray();
        if self.hidden {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        } else {
            // A desktop without a tray must retain its taskbar/dock recovery path.
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn request_quit(&mut self, ctx: &egui::Context) {
        if self.busy
            || self.loading
            || !self.approvals.is_empty()
            || self
                .sessions
                .iter()
                .any(|s| s.busy || s.loading || !s.approvals.is_empty())
        {
            self.show_main_window(ctx);
            self.show_exit_confirmation = true;
        } else {
            self.exiting = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn lifecycle_logic(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.exiting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_main_window(ctx);
        }
        for _ in 0..32 {
            match self.desktop.next_action() {
                Some(crate::desktop::Action::Show) => self.show_main_window(ctx),
                Some(crate::desktop::Action::Hide) => self.hide_main_window(ctx),
                Some(crate::desktop::Action::Quit) => self.request_quit(ctx),
                Some(crate::desktop::Action::Problem(error)) => {
                    self.desktop.last_error = Some(error.clone());
                    self.diagnostics
                        .push_back(format!("Desktop: {}", clipped(&error, 1800)));
                    if self.diagnostics.len() > 40 {
                        self.diagnostics.pop_front();
                    }
                }
                None => break,
            }
        }
    }

    fn drain_backend(&mut self, ctx: &egui::Context) {
        let mut saturated = true;
        // Even a minimized window drains events, so bounded queues never depend on visibility.
        for _ in 0..128 {
            let event = self
                .backend
                .as_ref()
                .and_then(|b| b.incoming.try_recv().ok());
            match event {
                Some(Event::Message(v)) => self.process(v),
                Some(Event::Diagnostic(s)) => {
                    self.diagnostics.push_back(clipped(&s, 2000));
                    if self.diagnostics.len() > 40 {
                        self.diagnostics.pop_front();
                    }
                }
                Some(Event::Closed(reason)) => {
                    self.disconnect();
                    self.error = Some(reason);
                    saturated = false;
                    break;
                }
                None => {
                    saturated = false;
                    break;
                }
            }
        }
        if saturated {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if self.ready
            && !self.loading
            && let Some(recent) = self.resume_after_connect.take()
        {
            self.resume(recent);
        }
    }

    fn tick_logic(&mut self, ctx: &egui::Context) {
        self.lifecycle_logic(ctx);
        if let Some(journal) = &self.journal {
            while let Ok(event) = journal.rx.try_recv() {
                match event {
                    crate::usage::DiskEvent::Loaded(records) => self.meter.merge(records),
                    crate::usage::DiskEvent::Error(error) => self.usage_error = Some(error),
                }
            }
        }
        self.drain_backend(ctx);
        let active_project = self.settings.active_project;
        let mut sessions = std::mem::take(&mut self.sessions);
        self.background_processing = true;
        for session in &mut sessions {
            session.swap(self);
            self.drain_backend(ctx);
            if self.settings.active_project != active_project {
                self.suspend_idle_connection();
            }
            session.swap(self);
        }
        self.background_processing = false;
        self.sessions.extend(sessions);
        if !self.discarded_threads.is_empty() {
            self.sessions.retain(|s| {
                !s.thread_id
                    .as_ref()
                    .is_some_and(|id| self.discarded_threads.contains(id))
            });
            if self
                .thread_id
                .as_ref()
                .is_some_and(|id| self.discarded_threads.contains(id))
            {
                self.disconnect();
                self.transcript = Transcript::default();
            }
            self.discarded_threads.clear();
        }
        // Keep idle cached conversations bounded; running work is never evicted.
        while self.sessions.len() > 20 {
            let Some(index) = self.sessions.iter().position(|s| {
                !s.busy && !s.loading && s.approvals.is_empty() && s.pending.is_empty()
            }) else {
                break;
            };
            self.sessions.remove(index);
        }
        if self.deferred_save {
            self.deferred_save = false;
            self.save();
        }
        self.desktop.update(self.settings.language, &self.status);
    }

    fn exit_ui(&mut self, ctx: &egui::Context) {
        if !self.show_exit_confirmation {
            return;
        }
        let lang = self.settings.language;
        let response = egui::Modal::new(egui::Id::new("quit_confirmation")).show(ctx, |ui| {
            ui.set_max_width((ctx.content_rect().width() - 48.0).clamp(120.0, 340.0));
            ui.strong(lang.text("앱을 종료할까요?"));
            ui.label(lang.text("종료하면 진행 중인 작업이 중단됩니다."));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button(lang.text("계속 실행")).clicked() {
                    self.show_exit_confirmation = false;
                }
                if ui.button(lang.text("작업 중단 후 종료")).clicked() {
                    self.exiting = true;
                    self.show_exit_confirmation = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
        if response.should_close() {
            self.show_exit_confirmation = false;
        }
    }

    fn main_ui(&mut self, ui: &mut egui::Ui) {
        let lang = self.settings.language;
        let ctx = ui.ctx().clone();
        let compact = ctx.content_rect().height() < 360.0;
        let project = self
            .settings
            .active()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "프로젝트".into());
        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Codex Mini").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(lang.text("설정"))
                                    .frame(true)
                                    .selected(self.show_settings),
                            )
                            .clicked()
                        {
                            self.show_settings = !self.show_settings;
                        }
                        ui.menu_button(lang.text("앱"), |ui| {
                            if ui.button(lang.text("백그라운드로 보내기"))
                                .on_hover_text(lang.text(if self.desktop.has_tray() {
                                    "창을 닫아도 작업은 계속됩니다. 트레이에서 다시 열 수 있습니다."
                                } else {
                                    "트레이를 사용할 수 없어 창을 닫으면 최소화합니다. 작업은 계속됩니다."
                                })).clicked() {
                                self.desktop.request(crate::desktop::Action::Hide);
                                ui.close();
                            }
                            if ui.button(lang.text("앱 종료")).clicked() {
                                self.desktop.request(crate::desktop::Action::Quit);
                                ui.close();
                            }
                        });
                        ui.toggle_value(&mut self.show_projects, lang.text("프로젝트"));
                        ui.menu_button(lang.text("대화"), |ui| {
                            let mut activate = None;
                            for (index, session) in self.sessions.iter().enumerate() {
                                let project = self.settings.projects.iter()
                                    .find(|p| Some(p.id) == session.project_id)
                                    .map(|p| p.name.as_str()).unwrap_or("Codex Mini");
                                let title = session.thread_id.as_ref()
                                    .and_then(|id| self.settings.recent.iter().find(|r| &r.id == id))
                                    .map(|r| r.title.as_str()).unwrap_or_else(|| lang.text("새 대화"));
                                let label = format!("{} · {} · {}", project, title, lang.message(&session.status));
                                if ui.add(egui::Button::new(&label).wrap_mode(egui::TextWrapMode::Truncate))
                                    .on_hover_text(&label).clicked() {
                                    activate = Some(index);
                                }
                            }
                            if let Some(index) = activate {
                                self.activate_session(index);
                                ui.close();
                            }
                            if !self.sessions.is_empty() {
                                ui.separator();
                            }
                            if ui
                                .add_enabled(
                                    self.ready || self.backend.is_some(),
                                    egui::Button::new(lang.text("새 대화")),
                                )
                                .clicked()
                            {
                                self.new_chat();
                                ui.close();
                            }
                            ui.separator();
                            let mut chosen = None;
                            let mut action = None;
                            if self.settings.recent.is_empty() {
                                ui.weak(lang.text("최근 대화 없음"));
                            }
                            for recent in self
                                .settings
                                .recent
                                .iter()
                                .filter(|r| r.project_id == self.settings.active_project)
                            {
                                let response = ui
                                    .add_enabled(
                                        self.ready || self.backend.is_some(),
                                        egui::Button::new(&recent.title).frame(false),
                                    )
                                    .on_hover_text(&recent.cwd);
                                response.context_menu(|ui| {
                                    action = self.chat_context_menu(ui, recent);
                                });
                                if response.clicked() {
                                    chosen = Some(recent.clone());
                                }
                            }
                            if let Some(action) = action {
                                self.apply_chat_action(action);
                                ui.close();
                            }
                            if let Some(recent) = chosen {
                                self.resume(recent);
                                ui.close();
                            }
                            if let Some(recent) = self
                                .settings
                                .recent
                                .iter()
                                .find(|r| Some(&r.id) == self.thread_id.as_ref())
                                .cloned()
                            {
                                let mut action = None;
                                ui.menu_button(lang.text("대화 관리"), |ui| {
                                    action = self.chat_context_menu(ui, &recent);
                                });
                                if let Some(action) = action {
                                    self.apply_chat_action(action);
                                    ui.close();
                                }
                            }
                            if let Some(id) = &self.thread_id {
                                ui.separator();
                                if ui.button(lang.text("대화 ID 복사")).clicked() {
                                    ctx.copy_text(id.clone());
                                    ui.close();
                                }
                            }
                        });
                    });
                });
            });
        if self.show_projects && ctx.content_rect().width() >= 620.0 {
            egui::Panel::left("projects")
                .default_size(176.0)
                .size_range(150.0..=240.0)
                .resizable(true)
                .frame(
                    egui::Frame::new()
                        .fill(ui.visuals().window_fill)
                        .inner_margin(8.0),
                )
                .show(ui, |ui| self.projects_ui(ui));
        }
        egui::Panel::bottom("status")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(lang.message(&self.status))
                                .small()
                                .color(ui.visuals().weak_text_color()),
                        )
                        .truncate(),
                    )
                    .on_hover_text(lang.message(&self.status));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.backend.is_none() {
                            if ui
                                .add(
                                    egui::Button::new(lang.text("연결"))
                                        .min_size(egui::vec2(44.0, 20.0)),
                                )
                                .clicked()
                            {
                                self.connect(&ctx);
                            }
                        } else if ui
                            .add_enabled(
                                !self.busy,
                                egui::Button::new(lang.text("연결 종료")).frame(false),
                            )
                            .clicked()
                        {
                            self.disconnect();
                        }
                    });
                });
            });
        let composer = egui::Panel::bottom("composer")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ui, |ui| {
                if let Some(error) = self.error.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(ui.visuals().error_fg_color, lang.message(&error));
                        if ui.small_button(lang.text("닫기")).clicked() {
                            self.error = None;
                        }
                    });
                }
                self.chat_options_ui(ui);
                egui::Frame::new()
                    .fill(ui.visuals().window_fill)
                    .stroke(egui::Stroke::new(
                        1.0,
                        ui.visuals().widgets.noninteractive.bg_stroke.color,
                    ))
                    .corner_radius(8)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        let shortcut = self.prompt_ui(ui, compact);
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let enabled = self.can_send()
                                        && !self.prompt.trim().is_empty()
                                        && !self.prompt_composing;
                                    let send = egui::Button::new(
                                        RichText::new(lang.text("보내기")).color(if enabled {
                                            ui.visuals().selection.stroke.color
                                        } else {
                                            ui.visuals().weak_text_color()
                                        }),
                                    )
                                    .fill(if enabled {
                                        ui.visuals().selection.bg_fill
                                    } else {
                                        ui.visuals().widgets.inactive.bg_fill
                                    });
                                    if ui
                                        .add_enabled(enabled, send)
                                        .on_disabled_hover_text(lang.text(if self.compacting {
                                            "컨텍스트 압축이 끝난 뒤 보낼 수 있습니다."
                                        } else if self.loading || self.busy {
                                            "대화 준비가 끝나면 보낼 수 있습니다."
                                        } else if !self.ready {
                                            "Codex에 먼저 연결하세요."
                                        } else {
                                            "아래에 작업 내용을 입력하세요."
                                        }))
                                        .on_hover_text(lang.text(if self.busy {
                                            "현재 작업에 메시지를 바로 추가합니다."
                                        } else {
                                            "Enter 전송 · Shift+Enter 줄바꿈"
                                        }))
                                        .clicked()
                                        || (enabled && shortcut)
                                    {
                                        self.submit();
                                    }
                                    if self.busy
                                        && ui
                                            .add_enabled(
                                                self.turn_id.is_some(),
                                                egui::Button::new(lang.text("중단")),
                                            )
                                            .clicked()
                                        && let (Some(thread), Some(turn)) =
                                            (&self.thread_id, &self.turn_id)
                                    {
                                        self.rpc(
                                            "turn/interrupt",
                                            json!({"threadId":thread,"turnId":turn}),
                                            Pending::Interrupt,
                                        );
                                    }
                                },
                            );
                        });
                    });
            });
        let height = composer.response.rect.height();
        if (height - self.composer_height).abs() > 0.5 {
            self.composer_height = height;
            // egui 0.35 stores a bottom panel's measured size for the next frame.
            ctx.request_repaint();
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(ui.visuals().panel_fill).inner_margin(egui::Margin::symmetric(10, 8)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(RichText::new(&project).size(13.0).strong()).truncate()).on_hover_text(&self.settings.cwd);

                });
                if !compact {
                    ui.add(egui::Label::new(RichText::new(&self.settings.cwd).small().color(ui.visuals().weak_text_color())).truncate()).on_hover_text(&self.settings.cwd);
                }
                ui.separator();
                egui::ScrollArea::vertical().id_salt("chat").stick_to_bottom(!self.transcript.messages.is_empty()).auto_shrink([false,false]).show(ui,|ui| {
                    if self.transcript.messages.is_empty() {
                        ui.add_space(if compact { 0.0 } else { 18.0 });
                        ui.strong(lang.text("대화 없음"));
                        if !compact {
                            ui.add_space(4.0);
                            ui.label(RichText::new(if self.backend.is_none() { lang.text("연결 후 메시지를 입력하세요.") } else { lang.text("아래에 작업 내용을 입력하세요.") }).color(ui.visuals().weak_text_color()));
                        }
                    }
                    if self.transcript.trimmed { ui.weak(lang.text("이전 내용 일부는 화면에서 생략했습니다. 원본은 Codex에 남아 있습니다.")); }
                    for m in &self.transcript.messages {
                        ui.push_id(&m.id, |ui| {
                            if m.tool {
                                let body = if m.id.starts_with("note-") && m.title == "지원하지 않는 요청" {
                                    std::borrow::Cow::Owned(lang.format("{method}\n요청을 실행하지 않았습니다. 필요한 경우 공식 Codex CLI에서 이어가세요.", &[&m.body]))
                                } else {
                                    std::borrow::Cow::Borrowed(m.body.as_str())
                                };
                                egui::CollapsingHeader::new(lang.message(&m.title)).id_salt("message").show(ui,|ui| {
                                    ui.add(egui::Label::new(RichText::new(body.as_ref()).monospace()).wrap().selectable(true));
                                    if ui.small_button(lang.text("복사")).clicked() { ctx.copy_text(body.into_owned()); }
                                });
                            } else {
                                chat_message_ui(ui, m, lang);
                                ui.add_space(8.0);
                            }
                        });
                    }
                });
            });
        if self.show_projects
            && ctx.content_rect().width() < 620.0
            && !self.global.open
            && !self.show_settings
            && self.project_editor.is_none()
            && self.approvals.is_empty()
        {
            let mut open = true;
            egui::Window::new(lang.text("프로젝트 목록"))
                .id(egui::Id::new("project_list"))
                .open(&mut open)
                .default_width(220.0)
                .max_height((ctx.content_rect().height() - 70.0).max(100.0))
                .vscroll(true)
                .show(&ctx, |ui| self.projects_ui(ui));
            if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_projects = false;
            }
        }
        self.project_editor_ui(&ctx);
        let global_was_open = self.global.open;
        if let Some(action) = self.global.ui(
            &ctx,
            self.ready,
            !self.busy && !self.loading && !self.sessions.iter().any(|s| s.busy || s.loading),
            &self.binary,
            lang,
        ) {
            match action {
                crate::global::Action::Binary(binary) => {
                    self.binary = binary;
                    self.save();
                    self.global.message =
                        "실행 경로를 저장했습니다. 다음 연결부터 적용됩니다.".into();
                }
                crate::global::Action::RefreshSkills => {
                    self.rpc(
                        "skills/list",
                        json!({"cwds":[self.settings.cwd],"forceReload":true}),
                        Pending::Skills,
                    );
                }
                crate::global::Action::Skill(path, enabled) => {
                    self.rpc(
                        "skills/config/write",
                        json!({"path":path,"enabled":enabled}),
                        Pending::SkillSaved,
                    );
                }
            }
        }
        if global_was_open && !self.global.open {
            self.save();
        }

        if self.show_settings {
            let mut open = true;
            egui::Window::new(lang.text("환경 설정"))
                .id(egui::Id::new("settings"))
                .open(&mut open)
                .collapsible(false)
                .fixed_size(egui::vec2(
                    (ctx.content_rect().width() - 40.0).min(400.0),
                    (ctx.content_rect().height() - 60.0).min(340.0),
                ))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .resizable(false)
                .vscroll(true)
                .show(&ctx, |ui| {
                    egui::Frame::new().inner_margin(6).show(ui, |ui| {
                        self.settings_ui(ui);
                        ui.add_space(4.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            let width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                            if ui
                                .add_sized(
                                    [width, 26.0],
                                    egui::Button::new(lang.text("사용량·절약")),
                                )
                                .clicked()
                            {
                                self.show_usage = true;
                                self.show_settings = false;
                                self.mcp_draft = self
                                    .settings
                                    .active()
                                    .map(|p| p.efficiency.disabled_mcp.join(", "))
                                    .unwrap_or_default();
                            }
                            if ui
                                .add_sized([width, 26.0], egui::Button::new(lang.text("전역 설정")))
                                .on_hover_text(lang.text("전역 설정 · 지침 / 스킬 / MCP"))
                                .clicked()
                            {
                                self.global.open = true;
                                self.show_settings = false;
                            }
                        });
                        if self.global.changed
                            && !self.busy
                            && !self.loading
                            && ui.button(lang.text("변경 적용 · 다시 연결")).clicked()
                        {
                            self.disconnect();
                            self.connect(&ctx);
                            self.global.changed = false;
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.checkbox(&mut self.show_diagnostics, lang.text("진단"));
                                    let account = if self.account.is_empty() {
                                        lang.text("연결 전").into()
                                    } else {
                                        lang.message(&self.account)
                                    };
                                    ui.with_layout(
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(account)
                                                        .small()
                                                        .color(ui.visuals().weak_text_color()),
                                                )
                                                .truncate(),
                                            );
                                        },
                                    );
                                },
                            );
                        });
                        if self.ready
                            && self.account == "로그인 필요"
                            && ui.button(lang.text("ChatGPT 로그인")).clicked()
                        {
                            self.rpc(
                                "account/login/start",
                                json!({"type":"chatgpt"}),
                                Pending::Login,
                            );
                        }
                    });
                });
            if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_settings = false;
            }
        }
        if let Some(url) = &self.auth_url {
            egui::Window::new(lang.text("로그인"))
                .id(egui::Id::new("login"))
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.hyperlink_to(lang.text("브라우저에서 ChatGPT 로그인 열기"), url);
                });
        }
        self.usage_window(&ctx);
        self.approval_ui(&ctx);
        self.delete_chat_ui(&ctx);
        if self.show_diagnostics {
            egui::Window::new(lang.text("진단"))
                .id(egui::Id::new("diagnostics"))
                .open(&mut self.show_diagnostics)
                .default_width(600.0)
                .show(&ctx, |ui| {
                    ui.label(format!(
                        "GUI PID: {} · Codex PID: {}",
                        std::process::id(),
                        self.backend
                            .as_ref()
                            .map(|b| b.pid().to_string())
                            .unwrap_or_else(|| lang.text("연결 전").into())
                    ));
                    ui.label(lang.format("Codex 실행 파일: {}", &[&(self.binary)]))
                        .on_hover_text(lang.text("로그는 최대 40줄만 메모리에 보관합니다."));

                    egui::ScrollArea::vertical()
                        .max_height(350.0)
                        .show(ui, |ui| {
                            for line in &self.diagnostics {
                                ui.add(
                                    egui::Label::new(RichText::new(line).monospace().size(10.0))
                                        .wrap()
                                        .selectable(true),
                                );
                            }
                        });
                });
        }
        self.exit_ui(&ctx);
    }
}

impl eframe::App for MiniApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick_logic(ctx);
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.main_ui(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save();
        self.disconnect();
        for mut session in std::mem::take(&mut self.sessions) {
            session.swap(self);
            self.disconnect();
            session.swap(self);
        }
    }
}

fn chat_message_ui(ui: &mut egui::Ui, message: &state::Message, lang: Language) -> egui::Rect {
    let user = message.title == "나";
    let width = if user {
        ui.available_width() * 0.80
    } else {
        ui.available_width()
    };
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(width, 0.0),
            egui::Layout::top_down(if user {
                egui::Align::Max
            } else {
                egui::Align::Min
            }),
            |ui| {
                egui::Frame::new()
                    .fill(if user {
                        ui.visuals().window_fill
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .corner_radius(6)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width((width - 20.0).max(1.0));
                        ui.horizontal(|ui| {
                            let title = RichText::new(lang.message(&message.title))
                                .small()
                                .strong()
                                .color(ui.visuals().weak_text_color());
                            if !user {
                                ui.label(title.clone());
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if user {
                                        ui.label(title);
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(lang.text("복사")).small(),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(message.body.clone());
                                    }
                                },
                            );
                        });
                        ui.add(
                            egui::Label::new(&message.body)
                                .wrap()
                                .selectable(true)
                                .halign(if user {
                                    egui::Align::Max
                                } else {
                                    egui::Align::Min
                                }),
                        );
                    })
                    .response
                    .rect
            },
        )
        .inner
    })
    .inner
}

fn token_grid(ui: &mut egui::Ui, id: &str, tokens: &crate::usage::Tokens, lang: Language) {
    egui::Grid::new(id).num_columns(2).show(ui, |ui| {
        for (label, value) in [
            ("입력", tokens.input_tokens),
            ("캐시 입력 (포함)", tokens.cached_input_tokens),
            ("출력", tokens.output_tokens),
            ("추론 출력 (포함)", tokens.reasoning_output_tokens),
        ] {
            ui.label(lang.text(label));
            ui.monospace(crate::usage::number(value));
            ui.end_row();
        }
    });
}

// Incomplete previews require a manual decision, even while auto-accept is on.
fn automatic_approval(a: &Approval) -> Option<Value> {
    if !a.preview_complete {
        return None;
    }
    match a.method.as_str() {
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval"
            if a.params["availableDecisions"]
                .as_array()
                .is_none_or(|ds| ds.iter().any(|v| v.as_str() == Some("accept"))) =>
        {
            Some(json!({"decision":"accept"}))
        }
        "item/permissions/requestApproval" if a.params["permissions"].is_object() => {
            Some(json!({"permissions":a.params["permissions"],"scope":"turn"}))
        }
        _ => None,
    }
}

fn approval_preview(params: &Value, previous: &str, lang: Language) -> (String, bool) {
    let mut preview = String::new();
    for (key, label) in [
        ("command", "실행할 명령"),
        ("cwd", "작업 폴더"),
        ("reason", "요청 이유"),
        ("grantRoot", "허용할 경로"),
    ] {
        if let Some(value) = params[key].as_str().filter(|s| !s.is_empty()) {
            preview.push_str(&format!("{}\n{value}\n\n", lang.text(label)));
        }
    }
    for (key, label) in [
        ("networkApprovalContext", "네트워크 접근"),
        ("permissions", "요청한 권한"),
        ("additionalPermissions", "추가 권한"),
    ] {
        if let Some(value) = params.get(key).filter(|v| !v.is_null()) {
            preview.push_str(&format!(
                "{}\n{}\n\n",
                lang.text(label),
                serde_json::to_string_pretty(value).unwrap_or_default()
            ));
        }
    }
    if !previous.is_empty() {
        preview.push_str(&lang.format("작업 내용\n{previous}", &[&previous]));
    }
    let complete = preview.len() <= state::MAX_BODY
        && !previous.contains("[화면 표시 생략]")
        && !preview.is_empty();
    (clipped(&preview, state::MAX_BODY), complete)
}

#[cfg(any(not(test), feature = "ui-review"))]
fn install_korean_font(ctx: &egui::Context) {
    let override_path = std::env::var("CODEX_MINI_FONT").ok();
    let candidates: Vec<&str> = override_path
        .iter()
        .map(String::as_str)
        .chain([
            "/usr/share/fonts/truetype/nanum/NanumGothic.ttf",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
            "C:\\Windows\\Fonts\\malgun.ttf",
        ])
        .collect();
    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            if data.len() > 32 * 1024 * 1024 {
                continue;
            }
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("korean".into(), egui::FontData::from_owned(data).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "korean".into());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("korean".into());
            ctx.set_fonts(fonts);
            break;
        }
    }
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod approval_tests;

#[cfg(test)]
#[path = "project_tests.rs"]
mod project_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn chat_fixture() -> MiniApp {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
        app.settings.projects[0].instructions = "보안 설계와 정확한 구현".into();
        app.models = vec![
            json!({"model":"test-model","isDefault":true,"defaultReasoningEffort":"medium","supportedReasoningEfforts":[{"reasoningEffort":"low"},{"reasoningEffort":"medium"},{"reasoningEffort":"high"}]}),
        ];
        app
    }

    #[test]
    fn chat_controls_choose_model_and_effort_even_during_response() {
        use egui_kittest::{Harness, kittest::Queryable};
        let app = chat_fixture();
        let mut h = Harness::builder()
            .with_size(egui::vec2(560.0, 400.0))
            .build_ui_state(|ui, app: &mut MiniApp| app.chat_options_ui(ui), app);
        h.run();
        h.get_by_value("Codex 기본값").click();
        h.run();
        h.get_by_label("권장");
        h.get_by_label("test-model").click();
        h.run();
        assert_eq!(h.state().settings.model, "test-model");
        h.state_mut().thread_id = Some("t".into());
        h.state_mut().busy = true;
        h.run();
        assert!(h.query_by_label("모델").is_none());
        h.get_by_value("기본값").click();
        h.run();
        h.get_by_label("high").click();
        h.run();
        assert_eq!(h.state().effective_effort(), "high");
        h.get_by_value("high").click();
        h.run();
        h.get_by_label("기본값").click();
        h.run();
        assert_eq!(h.state().effective_effort(), "medium");
        assert!(
            h.query_by_label("변경한 노력은 다음 전송부터 적용됩니다.")
                .is_none()
        );
        h.ctx
            .all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
        let effort = h.get_by_label("노력").rect().center();
        h.hover_at(effort);
        h.run();
        h.get_by_label("변경한 노력은 다음 전송부터 적용됩니다.");
        let mut settings = Harness::builder()
            .with_size(egui::vec2(560.0, 400.0))
            .build_ui_state(|ui, app: &mut MiniApp| app.settings_ui(ui), chat_fixture());
        settings.run();
        settings.get_by_label("작업 폴더");
        assert!(settings.query_by_label("모델").is_none());
        assert!(settings.query_by_label("추론").is_none());
        assert!(
            settings
                .query_by_label("추가 권한이 필요하면 승인을 요청합니다.")
                .is_none()
        );
        assert!(
            settings
                .query_by_label("작업 완료와 추가 질문을 시스템 알림으로 보냅니다.")
                .is_none()
        );
        settings
            .ctx
            .all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
        let notification = settings.get_by_label("작업 완료 알림").rect().center();
        settings.hover_at(notification);
        settings.run();
        settings.get_by_label("작업 완료와 추가 질문을 시스템 알림으로 보냅니다.");
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
    fn context_menu_moves_chat_and_delete_waits_for_server_success() {
        use egui_kittest::{Harness, kittest::Queryable};
        let mut app = chat_fixture();
        app.ready = true;
        app.settings.remember("t", "테스트 대화");
        let target = app
            .settings
            .add_project("대상 프로젝트", crate::transport::test_cwd(), "대상 지침")
            .unwrap();
        let mut h = Harness::builder()
            .with_size(egui::vec2(720.0, 520.0))
            .build_ui_state(|ui, app: &mut MiniApp| app.projects_ui(ui), app);
        h.run();
        h.get_by_label("테스트 대화").click_secondary();
        h.run();
        h.get_by_label_contains("다른 프로젝트로 이동").click();
        h.run();
        // The project navigation has the same name: use the last (submenu) entry.
        h.get_all_by_label("대상 프로젝트").last().unwrap().click();
        h.run();
        assert_eq!(h.state().settings.recent[0].project_id, Some(target));
        assert_eq!(
            h.state().settings.recent[0].cwd,
            crate::transport::test_cwd()
        );
        let app = h.state_mut();
        app.pending.insert(100, Pending::Delete("t".into()));
        app.loading = true;
        app.process(json!({"id":100,"error":{"message":"failure"}}));
        assert_eq!(app.settings.recent.len(), 1);
        assert!(!app.loading);
        app.pending.insert(101, Pending::Delete("t".into()));
        app.process(json!({"id":101,"result":{}}));
        assert!(app.settings.recent.is_empty());
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

    fn input_harness() -> egui_kittest::Harness<'static, MiniApp> {
        let mut app = chat_fixture();
        app.settings.projects[0].instructions.clear();
        app.show_projects = false;
        app.ready = true;
        app.backend = Some(
            Backend::start(
                &format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR")),
                crate::transport::test_cwd(),
                || {},
            )
            .unwrap(),
        );
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(560.0, 400.0))
            .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
        h.ctx
            .memory_mut(|m| m.request_focus(egui::Id::new("chat_prompt")));
        h.run();
        h
    }

    #[test]
    fn auto_accept_toggle_handles_pending_future_and_manual_requests() {
        use egui_kittest::kittest::Queryable;
        for zoom in [1.0, 1.5] {
            let mut h = input_harness();
            h.ctx.set_zoom_factor(zoom);
            h.state_mut().busy = true;
            h.state_mut()
                .server_request(json!({"id":"auto-test-command",
                "method":"item/commandExecution/requestApproval",
                "params":{"command":"printf hello","availableDecisions":["accept","decline"]}}));
            assert_eq!(h.state().approvals.len(), 1);
            h.run();
            h.get_by_label("자동수락: 꺼짐").focus();
            h.key_press(egui::Key::Space);
            h.run();
            assert!(h.state().auto_accept);
            assert!(h.state().approvals.is_empty());
            h.get_by_label("자동수락: 켜짐");
            for (id, method, params, expected) in [
                (
                    "auto-test-command",
                    "",
                    json!({}),
                    json!({"decision":"accept"}),
                ),
                (
                    "auto-test-file",
                    "item/fileChange/requestApproval",
                    json!({"reason":"update file"}),
                    json!({"decision":"accept"}),
                ),
                (
                    "auto-test-permissions",
                    "item/permissions/requestApproval",
                    json!({"permissions":{"network":{"enabled":true}}}),
                    json!({"permissions":{"network":{"enabled":true}},"scope":"turn"}),
                ),
            ] {
                if !method.is_empty() {
                    h.state_mut()
                        .server_request(json!({"id":id,"method":method,"params":params}));
                }
                let Event::Message(message) = h
                    .state()
                    .backend
                    .as_ref()
                    .unwrap()
                    .incoming
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap()
                else {
                    panic!("expected reply")
                };
                assert_eq!(message["params"]["id"], id);
                assert_eq!(message["params"]["result"], expected);
                assert!(h.state().approvals.is_empty());
            }
            // User questions, incomplete previews and unsupported decisions stay manual.
            for (id, method, params) in [
                (
                    "question",
                    "item/tool/requestUserInput",
                    json!({"questions":[{"id":"q","question":"Choose"}]}),
                ),
                (
                    "incomplete",
                    "item/commandExecution/requestApproval",
                    json!({}),
                ),
                (
                    "restricted",
                    "item/commandExecution/requestApproval",
                    json!({"command":"ls","availableDecisions":["decline"]}),
                ),
            ] {
                h.state_mut()
                    .server_request(json!({"id":id,"method":method,"params":params}));
            }
            assert_eq!(h.state().approvals.len(), 3);
            h.get_by_label("자동수락: 켜짐").focus();
            h.key_press(egui::Key::Space);
            h.run();
            assert!(!h.state().auto_accept);
            h.state_mut().server_request(json!({"id":"manual", "method":"item/fileChange/requestApproval", "params":{"reason":"review"}}));
            assert_eq!(h.state().approvals.len(), 4);
            h.state_mut().auto_accept = true;
            h.state_mut().disconnect();
            assert!(!h.state().auto_accept);
        }
    }

    #[test]
    fn followup_questions_notify_even_with_pending_requests_and_auto_accept() {
        for language in [Language::Korean, Language::English] {
            let mut app = chat_fixture();
            app.settings.language = language;
            app.hidden = true;
            app.server_request(json!({"id":"pending","method":"item/commandExecution/requestApproval","params":{}}));
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
            app.server_request(json!({"id":"quiet","method":"item/tool/requestUserInput","params":{"questions":[]}}));
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

    #[cfg(feature = "ui-review")]
    #[test]
    #[ignore = "render auto-accept controls at minimum window size"]
    fn render_auto_accept_review() {
        for theme in [
            crate::preferences::Theme::Dark,
            crate::preferences::Theme::Light,
        ] {
            for zoom in [1.0, 1.5] {
                let mut app = chat_fixture();
                app.show_projects = false;
                app.auto_accept = true;
                let mut h = egui_kittest::Harness::builder()
                    .with_size(egui::vec2(560.0, 400.0))
                    .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
                crate::preferences::configure(&h.ctx, theme);
                install_korean_font(&h.ctx);
                h.ctx.set_zoom_factor(zoom);
                h.run();
                h.render()
                    .unwrap()
                    .save(format!("/tmp/codex-mini-auto-{theme:?}-{zoom}.png"))
                    .unwrap();
                h.state_mut().show_settings = true;
                h.run();
                h.render()
                    .unwrap()
                    .save(format!(
                        "/tmp/codex-mini-help-settings-{theme:?}-{zoom}.png"
                    ))
                    .unwrap();
            }
        }
    }

    #[test]
    fn notification_preference_can_be_changed_with_keyboard_during_work() {
        use egui_kittest::{Harness, kittest::Queryable};
        for zoom in [1.0, 1.5] {
            let mut app = chat_fixture();
            app.busy = true;
            app.prompt = "초안 유지".into();
            let mut h = Harness::builder()
                .with_size(egui::vec2(560.0, 400.0))
                .build_ui_state(|ui, app: &mut MiniApp| app.settings_ui(ui), app);
            h.ctx.set_zoom_factor(zoom);
            h.run();
            h.get_by_label("작업 완료 알림").focus();
            h.key_press(egui::Key::Space);
            h.run();
            assert!(!h.state().settings.notifications);
            assert!(h.state().busy);
            assert_eq!(h.state().prompt, "초안 유지");
        }
    }

    #[cfg(feature = "ui-review")]
    #[test]
    #[ignore = "render background settings and quit confirmation"]
    fn render_background_review() {
        for theme in [
            crate::preferences::Theme::Dark,
            crate::preferences::Theme::Light,
        ] {
            for zoom in [1.0, 1.5] {
                let mut app = chat_fixture();
                app.show_projects = false;
                app.show_settings = true;
                app.desktop.test_tray = true;
                let mut h = egui_kittest::Harness::builder()
                    .with_size(egui::vec2(560.0, 400.0))
                    .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
                crate::preferences::configure(&h.ctx, theme);
                install_korean_font(&h.ctx);
                h.ctx.set_zoom_factor(zoom);
                h.run();
                h.render()
                    .unwrap()
                    .save(format!(
                        "/tmp/codex-mini-background-settings-{theme:?}-{zoom}.png"
                    ))
                    .unwrap();
                h.state_mut().show_settings = false;
                h.state_mut().show_exit_confirmation = true;
                h.state_mut().busy = true;
                h.run();
                h.render()
                    .unwrap()
                    .save(format!(
                        "/tmp/codex-mini-background-quit-{theme:?}-{zoom}.png"
                    ))
                    .unwrap();
            }
        }
    }

    fn background_harness(tray: bool) -> egui_kittest::Harness<'static, MiniApp> {
        let mut app = chat_fixture();
        app.show_projects = false;
        app.desktop.test_tray = tray;
        app.ready = true;
        app.backend = Some(
            Backend::start(
                &format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR")),
                crate::transport::test_cwd(),
                || {},
            )
            .unwrap(),
        );
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(560.0, 400.0))
            .build_ui_state(
                |ui, app: &mut MiniApp| {
                    app.tick_logic(ui.ctx());
                    if !app.hidden {
                        app.main_ui(ui);
                    }
                },
                app,
            )
    }

    #[test]
    fn close_hides_and_tray_reopens_the_same_running_session() {
        use crate::desktop::Action;
        let mut h = background_harness(true);
        hold_turn(h.state_mut());
        h.state_mut().prompt = "작성 중인 초안".into();
        let pid = h.state().backend.as_ref().unwrap().pid();
        h.run();
        h.input_mut()
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        h.step();
        assert!(h.state().hidden && h.state().busy);
        let commands = &h.output().viewport_output[&egui::ViewportId::ROOT].commands;
        assert!(commands.contains(&egui::ViewportCommand::CancelClose));
        assert!(commands.contains(&egui::ViewportCommand::Visible(false)));
        h.state_mut().notification(json!({"method":"item/agentMessage/delta","params":{"threadId":"mock-thread","itemId":"answer","delta":"백그라운드 응답"}}));
        h.state().desktop.request(Action::Show);
        h.run();
        assert!(!h.state().hidden);
        assert!(h.state().busy);
        assert_eq!(h.state().backend.as_ref().unwrap().pid(), pid);
        assert_eq!(h.state().turn_id.as_deref(), Some("mock-turn"));
        assert_eq!(h.state().prompt, "작성 중인 초안");
        assert!(
            h.state()
                .transcript
                .messages
                .iter()
                .any(|m| m.body == "백그라운드 응답")
        );
    }

    #[test]
    fn backend_streams_and_finishes_while_window_is_hidden() {
        let mut h = background_harness(true);
        h.state_mut().hidden = true;
        h.state_mut().thread_id = Some("mock-thread".into());
        h.state_mut().start_turn("hello background".into());
        let pid = h.state().backend.as_ref().unwrap().pid();
        let deadline = Instant::now() + Duration::from_secs(4);
        while h.state().busy {
            assert!(Instant::now() < deadline);
            h.step();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(h.state().hidden);
        assert_eq!(h.state().backend.as_ref().unwrap().pid(), pid);
        assert_eq!(h.state().desktop.sent.len(), 1);
        assert!(
            h.state()
                .transcript
                .messages
                .iter()
                .any(|m| m.body.contains("안녕하세요."))
        );
        h.state().desktop.request(crate::desktop::Action::Show);
        h.run();
        assert!(!h.state().hidden);
        assert!(h.state().ready);
    }

    #[test]
    fn missing_tray_minimizes_and_quit_confirms_before_stopping_work() {
        use crate::desktop::Action;
        use egui_kittest::kittest::Queryable;
        let mut h = background_harness(false);
        hold_turn(h.state_mut());
        h.state().desktop.request(Action::Hide);
        h.step();
        assert!(!h.state().hidden);
        assert!(
            h.output().viewport_output[&egui::ViewportId::ROOT]
                .commands
                .contains(&egui::ViewportCommand::Minimized(true))
        );
        h.state().desktop.request(Action::Quit);
        h.run();
        assert!(h.state().show_exit_confirmation && !h.state().exiting);
        h.get_by_label("계속 실행").click();
        h.run();
        assert!(!h.state().show_exit_confirmation && h.state().busy);
        h.state().desktop.request(Action::Quit);
        h.run();
        h.key_press(egui::Key::Escape);
        h.run();
        assert!(!h.state().show_exit_confirmation && !h.state().exiting);
        h.state().desktop.request(Action::Quit);
        h.run();
        h.get_by_label("작업 중단 후 종료").click();
        h.step();
        assert!(h.state().exiting);
        h.state_mut().disconnect();
        assert!(h.state().backend.is_none());
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

    #[test]
    fn working_chat_accepts_enter_and_button_without_duplicate_messages() {
        use egui_kittest::kittest::Queryable;
        let mut h = input_harness();
        hold_turn(h.state_mut());
        let started = h.state().started;
        h.state_mut().prompt = "동일한 추가 지침".into();
        h.run();
        h.key_press(egui::Key::Enter);
        h.run();
        assert_eq!(h.state().transcript.messages.len(), 2);
        assert!(h.state().prompt.is_empty());
        h.state_mut().prompt = "동일한 추가 지침".into();
        h.run();
        h.get_by_label("보내기").click();
        h.run();
        assert_eq!(h.state().transcript.messages.len(), 3);
        receive_until(h.state_mut(), |app| {
            app.pending.is_empty() && app.transcript.messages.iter().any(|m| m.id == "user-3")
        });
        assert_eq!(
            h.state()
                .transcript
                .messages
                .iter()
                .filter(|m| m.body == "동일한 추가 지침")
                .count(),
            2
        );
        assert_eq!(h.state().turn_id.as_deref(), Some("mock-turn"));
        assert!(h.state().busy);
        assert_eq!(h.state().started, started);
        assert_eq!(h.state().submitted_prompt, "hold");
        assert!(h.state().meter.records.is_empty());
    }

    #[test]
    fn rejected_followup_preserves_new_draft_and_completion_race() {
        let mut h = input_harness();
        hold_turn(h.state_mut());
        let app = h.state_mut();
        app.prompt = "reject".into();
        app.submit();
        app.prompt = "새로 작성 중인 내용".into();
        receive_until(app, |app| app.pending.is_empty());
        assert_eq!(app.prompt, "새로 작성 중인 내용");
        assert!(app.busy);
        assert!(
            app.transcript
                .messages
                .iter()
                .any(|m| m.tool && m.body == "reject")
        );
        assert!(
            !app.transcript
                .messages
                .iter()
                .any(|m| m.title == "나" && m.body == "reject")
        );
        app.prompt = "reject".into();
        app.submit();
        app.notification(json!({"method":"turn/completed","params":{"threadId":"mock-thread","turn":{"id":"mock-turn","status":"completed"}}}));
        receive_until(app, |app| app.pending.is_empty());
        assert_eq!(app.prompt, "reject");
        assert!(!app.busy);
        assert!(app.turn_id.is_none());
        app.compacting = true;
        app.busy = true;
        app.turn_id = Some("compact".into());
        app.submit();
        assert!(
            app.pending.is_empty(),
            "manual compaction cannot be steered"
        );
    }

    #[test]
    fn projects_can_be_added_during_work_without_switching_or_disconnect() {
        use egui_kittest::{Harness, kittest::Queryable};
        for loading in [false, true] {
            let mut app = chat_fixture();
            app.busy = !loading;
            app.loading = loading;
            app.thread_id = Some("running-thread".into());
            app.turn_id = Some("running-turn".into());
            app.prompt = "작성 중인 내용".into();
            app.show_projects = true;
            app.backend = Some(
                Backend::start(
                    &format!("{}/tests/mock_codex.py", env!("CARGO_MANIFEST_DIR")),
                    crate::transport::test_cwd(),
                    || {},
                )
                .unwrap(),
            );
            let backend_pid = app.backend.as_ref().unwrap().pid();
            let previous = app.settings.active_project;
            let cwd = app.settings.cwd.clone();
            let mut h = Harness::builder()
                .with_size(egui::vec2(720.0, 520.0))
                .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
            h.run();
            h.get_by_label("추가").click();
            h.run();
            let editor = h.state_mut().project_editor.as_mut().unwrap();
            editor.name = "작업 중 추가".into();
            editor.cwd = std::env::temp_dir().display().to_string();
            h.run();
            h.get_by_label("저장").click();
            h.run();
            assert!(h.state().project_editor.is_none());
            assert_eq!(h.state().settings.projects.len(), 2);
            assert_eq!(h.state().settings.active_project, previous);
            assert_eq!(h.state().settings.cwd, cwd);
            assert_eq!(h.state().thread_id.as_deref(), Some("running-thread"));
            assert_eq!(h.state().turn_id.as_deref(), Some("running-turn"));
            assert_eq!(h.state().backend.as_ref().unwrap().pid(), backend_pid);
            assert_eq!(h.state().prompt, "작성 중인 내용");
            assert_eq!(h.state().loading, loading);
            assert_eq!(h.state().busy, !loading);
        }
    }

    #[cfg(feature = "ui-review")]
    #[test]
    #[ignore = "render minimum-size aligned chat bubbles"]
    fn render_aligned_bubbles_review() {
        for theme in [
            crate::preferences::Theme::Dark,
            crate::preferences::Theme::Light,
        ] {
            for zoom in [1.0, 1.5] {
                let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(560.0, 400.0))
                    .build_ui(|ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            for (id, title, body) in [
                                ("user", "나", "작업 중에도 추가 요청을 바로 반영해 줘.\n한글과 줄바꿈도 오른쪽에 표시해 줘."),
                                ("agent", "CODEX", "현재 작업에 추가 요청을 반영하고 있습니다."),
                            ] {
                                ui.push_id(id, |ui| chat_message_ui(ui, &state::Message {
                                    id: id.into(), title: title.into(), body: body.into(), tool: false,
                                }, Language::Korean));
                                ui.add_space(8.0);
                            }
                        });
                    });
                crate::preferences::configure(&h.ctx, theme);
                install_korean_font(&h.ctx);
                h.ctx.set_zoom_factor(zoom);
                h.run();
                h.render()
                    .unwrap()
                    .save(format!("/tmp/codex-mini-aligned-{theme:?}-{zoom}.png"))
                    .unwrap();
            }
        }
    }

    #[test]
    fn user_bubble_and_text_align_right_at_minimum_size_and_zoom() {
        use egui_kittest::Harness;
        for zoom in [1.0, 1.5] {
            let mut h = Harness::builder()
                .with_size(egui::vec2(560.0, 400.0))
                .build_ui_state(
                    |ui, rects: &mut Vec<egui::Rect>| {
                        rects.clear();
                        for (id, title, body) in [
                            ("u", "나", "한글 메시지\n줄바꿈도 오른쪽"),
                            ("a", "CODEX", "응답은 왼쪽"),
                        ] {
                            rects.push(chat_message_ui(
                                ui,
                                &state::Message {
                                    id: id.into(),
                                    title: title.into(),
                                    body: body.into(),
                                    tool: false,
                                },
                                Language::Korean,
                            ));
                        }
                    },
                    Vec::new(),
                );
            h.ctx.set_zoom_factor(zoom);
            h.run();
            let rects = h.state();
            assert!(
                (rects[0].right() - rects[1].right()).abs() < 1.0,
                "{rects:?}"
            );
            assert!(
                rects[0].left() > rects[1].left() + rects[1].width() * 0.15,
                "{rects:?}"
            );
            assert!(rects[0].top() < rects[1].top());
        }
    }

    #[test]
    fn enter_sends_and_shift_enter_inserts_a_newline() {
        let mut h = input_harness();
        h.state_mut().prompt = "first".into();
        h.run();
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Enter);
        h.run();
        assert!(h.state().prompt.contains('\n'));
        assert!(h.state().pending.is_empty());
        let draft = h.state().prompt.clone();
        h.key_press(egui::Key::Enter);
        h.run();
        assert_eq!(h.state().queued_prompt.as_deref(), Some(draft.as_str()));
        assert!(h.state().loading);
        assert!(h.state().project_editor.is_none());
    }

    #[test]
    fn korean_preedit_is_inline_and_enter_sends_only_committed_text() {
        let mut h = input_harness();
        for text in ["ㅎ", "하", "한"] {
            h.input_mut()
                .events
                .push(egui::Event::Ime(egui::ImeEvent::Preedit {
                    text: text.into(),
                    active_range_chars: Some(1..1),
                }));
            h.run();
            assert_eq!(
                h.state().prompt,
                text,
                "composition must be visible before commit"
            );
            assert!(h.state().prompt_composing);
        }
        h.key_press(egui::Key::Enter);
        h.run();
        assert!(h.state().pending.is_empty());
        assert_eq!(h.state().prompt, "한");
        h.input_mut()
            .events
            .push(egui::Event::Ime(egui::ImeEvent::Commit("한".into())));
        h.key_press(egui::Key::Enter);
        h.run();
        assert!(!h.state().prompt_composing);
        assert_eq!(h.state().queued_prompt.as_deref(), Some("한"));
    }

    #[cfg(feature = "ui-review")]
    #[test]
    #[ignore = "render Korean composition for visual review"]
    fn render_inline_korean_review() {
        let mut h = input_harness();
        crate::preferences::configure(&h.ctx, crate::preferences::Theme::Dark);
        install_korean_font(&h.ctx);
        h.input_mut()
            .events
            .push(egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "한".into(),
                active_range_chars: Some(1..1),
            }));
        h.run();
        assert_eq!(h.state().prompt, "한");
        h.render()
            .unwrap()
            .save("/tmp/codex-mini-inline-korean.png")
            .unwrap();
    }

    #[test]
    fn enter_outside_composer_does_not_send_the_draft() {
        use egui_kittest::kittest::Queryable;
        let mut h = input_harness();
        h.state_mut().prompt = "keep draft".into();
        h.get_by_label("설정").focus();
        h.key_press(egui::Key::Enter);
        h.run();
        assert!(h.state().show_settings);
        assert_eq!(h.state().prompt, "keep draft");
        assert!(h.state().pending.is_empty());
    }

    #[test]
    fn long_drafts_keep_send_and_navigation_visible_at_zoom() {
        use egui_kittest::{Harness, kittest::Queryable};
        for zoom in [1.0, 1.5] {
            let mut app = chat_fixture();
            app.prompt = "긴 작업 설명과 경로 /home/project/src/main.rs\n".repeat(100);
            app.ready = true;
            app.show_projects = false;
            let mut h = Harness::builder()
                .with_size(egui::vec2(560.0, 400.0))
                .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
            crate::preferences::configure(&h.ctx, crate::preferences::Theme::Dark);
            h.ctx.set_zoom_factor(zoom);
            h.run();
            for label in ["보내기", "설정", "연결", "대화 없음"] {
                let rect = h.get_by_label(label).rect();
                assert!(
                    rect.min.x >= 0.0
                        && rect.min.y >= 0.0
                        && rect.max.x <= 561.0
                        && rect.max.y <= 401.0,
                    "{label} at {zoom}: {rect:?}"
                );
            }
            h.get_by_label("설정").focus();
            h.key_press(egui::Key::Enter);
            h.run();
            assert!(h.state().show_settings);
            h.key_press(egui::Key::Escape);
            h.run();
            assert!(!h.state().show_settings);
            assert!(h.state().prompt.starts_with("긴 작업 설명"));
        }
    }

    #[cfg(feature = "ui-review")]
    #[test]
    #[ignore = "render optional visual review PNGs"]
    fn render_chat_review() {
        use egui_kittest::{Harness, kittest::Queryable};
        for theme in [
            crate::preferences::Theme::Dark,
            crate::preferences::Theme::Light,
        ] {
            for (zoom, size, conversation) in [
                (1.0, egui::vec2(560.0, 400.0), false),
                (1.5, egui::vec2(560.0, 400.0), false),
                (1.0, egui::vec2(900.0, 640.0), true),
                (1.5, egui::vec2(900.0, 640.0), true),
            ] {
                let mut app = chat_fixture();
                app.settings.theme = theme;
                app.show_projects = conversation;
                if conversation {
                    app.settings.projects[0].name = "Macbuntu".into();
                    app.settings.recent.push(Recent {
                        id: "review".into(),
                        title: "Linux 데스크톱 UI/UX 개선".into(),
                        cwd: app.settings.cwd.clone(),
                        project_id: app.settings.active_project,
                        model: "test-model".into(),
                        effort: String::new(),
                        efficiency: Default::default(),
                    });
                    app.thread_id = Some("review".into());
                    app.settings.model = "test-model".into();
                    app.transcript.put(
                        "user",
                        "나",
                        "한글 가독성을 높이고, 작은 창에서도 편하게 사용할 수 있도록 다듬어 줘.",
                        false,
                        false,
                    );
                    app.transcript.put(
                        "tool",
                        "명령 · 완료",
                        "$ cargo test --locked\n35 passed; 0 failed",
                        true,
                        false,
                    );
                    app.transcript.put("assistant", "Codex", "본문과 버튼의 글자 크기를 조정했습니다. 긴 메시지는 입력창 안에서 스크롤되며, 프로젝트 목록에서 현재 선택한 항목을 확인할 수 있습니다.\n\n라이트·다크 테마와 150% 확대에서도 주요 조작을 확인했습니다.", false, false);
                    app.prompt = "다음으로 키보드 조작도 확인해 줘.".into();
                }
                let mut h = Harness::builder()
                    .with_size(size)
                    .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
                crate::preferences::configure(&h.ctx, theme);
                install_korean_font(&h.ctx);
                h.ctx.set_zoom_factor(zoom);
                h.run();
                let name = format!("/tmp/codex-mini-chat-{theme:?}-{zoom}-{conversation}.png");
                h.render().unwrap().save(&name).unwrap();
                eprintln!("{name}");
                if conversation {
                    h.state_mut().show_projects = false;
                    h.run();
                    h.get_by_label("명령 · 완료").click();
                    h.run();
                    let name = format!("/tmp/codex-mini-tools-{theme:?}-{zoom}.png");
                    h.render().unwrap().save(&name).unwrap();
                    eprintln!("{name}");
                }
            }
        }
    }

    #[test]
    fn language_and_theme_switch_at_minimum_size_without_changing_content() {
        use crate::preferences::Theme;
        use egui_kittest::{Harness, kittest::Queryable};
        for zoom in [1.0, 1.5] {
            for language in [Language::Korean, Language::English] {
                for theme in [Theme::Dark, Theme::Light] {
                    let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
                    let settings = Settings {
                        language,
                        theme,
                        ..Default::default()
                    };
                    let mut app = MiniApp::new(&cc, settings, "unused".into());
                    app.show_settings = true;
                    app.busy = true; // Appearance changes must work during a running task.
                    app.prompt = "원본 prompt {text}".into();
                    app.transcript.put("user", "나", "설정", false, false);
                    app.status = "완료 · 15초".into();
                    let mut harness = Harness::builder()
                        .with_size(egui::vec2(560.0, 400.0))
                        .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
                    crate::preferences::configure(&harness.ctx, theme);
                    harness.ctx.set_zoom_factor(zoom);
                    harness.run();
                    let labels = ["언어", "테마", "작업 폴더"]
                        .map(|key| harness.get_by_label(language.text(key)).rect());
                    for label in &labels[1..] {
                        assert!(
                            (label.min.x - labels[0].min.x).abs() < 1.0,
                            "label columns: {labels:?}"
                        );
                    }
                    let korean = harness.get_by_label("한국어").rect();
                    let english = harness.get_by_label("English").rect();
                    let light = harness.get_by_label(language.text("라이트")).rect();
                    let dark = harness.get_by_label(language.text("다크")).rect();
                    assert!((korean.min.x - light.min.x).abs() < 1.0);
                    assert!((english.max.x - dark.max.x).abs() < 1.0);
                    assert!((korean.width() - english.width()).abs() < 1.0);
                    assert!((light.width() - dark.width()).abs() < 1.0);
                    let folder = harness
                        .get_by(|node| {
                            node.role() == egui::accesskit::Role::TextInput
                                && node.value().as_deref()
                                    == Some(harness.state().settings.cwd.as_str())
                        })
                        .rect();
                    assert!(
                        (folder.min.x - korean.min.x).abs() < 1.0,
                        "folder: {folder:?}, language: {korean:?}"
                    );
                    assert!(
                        (folder.max.x - english.max.x).abs() < 1.0,
                        "folder: {folder:?}, language: {english:?}"
                    );
                    let click = |h: &mut Harness<MiniApp>, label: &str| {
                        let rect = h.get_by_label(label).rect();
                        assert!(
                            rect.min.x >= 0.0 && rect.max.x <= 561.0 && rect.max.y <= 401.0,
                            "{label}: {rect:?}"
                        );
                        let point = rect.center() / h.ctx.pixels_per_point();
                        h.hover_at(point);
                        h.run();
                        h.drag_at(point);
                        h.run();
                        h.drop_at(point);
                        h.run();
                    };
                    let other_language = if language == Language::Korean {
                        Language::English
                    } else {
                        Language::Korean
                    };
                    click(
                        &mut harness,
                        if other_language == Language::English {
                            "English"
                        } else {
                            "한국어"
                        },
                    );
                    assert_eq!(harness.state().settings.language, other_language);
                    harness.get_by_label(other_language.text("테마"));
                    let other_theme = if theme == Theme::Dark {
                        Theme::Light
                    } else {
                        Theme::Dark
                    };
                    click(
                        &mut harness,
                        other_language.text(if other_theme == Theme::Light {
                            "라이트"
                        } else {
                            "다크"
                        }),
                    );
                    assert_eq!(harness.state().settings.theme, other_theme);
                    assert_eq!(
                        harness.ctx.global_style().visuals.dark_mode,
                        other_theme == Theme::Dark
                    );
                    assert!(harness.state().busy);
                    assert_eq!(harness.state().prompt, "원본 prompt {text}");
                    assert_eq!(harness.state().transcript.messages[0].body, "설정");
                    assert_eq!(harness.state().status, "완료 · 15초");
                    harness.key_press(egui::Key::Escape);
                    harness.run();
                    assert!(!harness.state().show_settings);
                    harness.get_by_label(other_language.message("완료 · 15초").as_ref());
                }
            }
        }
    }

    #[test]
    fn appearance_controls_work_with_keyboard_focus() {
        use egui_kittest::{Harness, kittest::Queryable};
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
        app.show_settings = true;
        let mut harness = Harness::builder()
            .with_size(egui::vec2(560.0, 400.0))
            .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
        crate::preferences::configure(&harness.ctx, crate::preferences::Theme::Dark);
        harness.run();
        harness.get_by_label("English").focus();
        harness.run();
        harness.key_press(egui::Key::Space);
        harness.run();
        assert_eq!(harness.state().settings.language, Language::English);
        harness.get_by_label("Light").focus();
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(
            harness.state().settings.theme,
            crate::preferences::Theme::Light
        );
    }

    #[test]
    fn approval_language_changes_preserve_payload_and_flag_incomplete_previews() {
        for lang in [Language::Korean, Language::English] {
            let payload = "echo '한글 작업 / {value}'";
            let (text, complete) =
                approval_preview(&json!({"command":payload, "cwd":"/프로젝트"}), "", lang);
            assert!(complete);
            assert!(text.starts_with(lang.text("실행할 명령")));
            assert!(text.contains(payload) && text.contains("/프로젝트"));
            assert!(
                !approval_preview(
                    &json!({"command":"x".repeat(state::MAX_BODY + 1)}),
                    "",
                    lang
                )
                .1
            );
        }
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
        app.server_request(json!({"id":1,"method":"item/commandExecution/requestApproval","params":{"command":"echo '원본'"}}));
        app.settings.language = Language::English;
        let _ = cc
            .egui_ctx
            .run_ui(Default::default(), |ui| app.approval_ui(ui.ctx()));
        assert_eq!(app.approvals.len(), 1);
        assert!(app.approvals[0].preview.starts_with("Command"));
        assert!(app.approvals[0].preview.contains("echo '원본'"));
        assert!(app.approvals[0].preview_complete);
    }

    #[test]
    fn project_can_be_created_from_navigation_and_edited() {
        use egui_kittest::{Harness, kittest::Queryable};
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
        app.show_projects = true;
        let mut harness = Harness::builder()
            .with_size(egui::vec2(720.0, 520.0))
            .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
        harness.ctx.set_style_of(
            egui::Theme::Dark,
            (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone(),
        );
        harness.run();
        harness.get_by_label("추가").click();
        harness.run();
        let editor = harness.state_mut().project_editor.as_mut().unwrap();
        editor.name = "UI 프로젝트".into();
        editor.cwd = std::env::temp_dir().to_string_lossy().into_owned();
        editor.instructions.clear();
        harness.run();
        harness.get_by_label("저장").click();
        harness.run();
        assert_eq!(
            harness.state().settings.active().unwrap().name,
            "UI 프로젝트"
        );
        assert!(harness.state().project_editor.is_none());
        harness.get_by_label("프로젝트 편집").click();
        harness.run();
        assert_eq!(
            harness
                .state()
                .project_editor
                .as_ref()
                .unwrap()
                .instructions,
            ""
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
        app.resume(first);
        assert_eq!(app.backend.as_ref().unwrap().pid(), first_pid);
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
        app.request_quit(&app.ui_context.clone());
        assert!(
            app.show_exit_confirmation,
            "background work requires the existing quit confirmation"
        );
    }

    #[test]
    fn project_and_new_chat_buttons_remain_available_during_work() {
        use egui_kittest::{Harness, kittest::Queryable};
        let mut app = live_worker();
        let second = app
            .settings
            .add_project("Second project", crate::transport::test_cwd(), "")
            .unwrap();
        let first_pid = app.backend.as_ref().unwrap().pid();
        app.show_projects = true;
        let mut h = Harness::builder()
            .with_size(egui::vec2(720.0, 520.0))
            .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
        h.run();
        h.get_by_label("새 대화").click();
        h.run();
        assert!(h.state().sessions[0].busy);
        assert!(h.state().backend.is_some());
        h.get_by_label("Second project").click();
        h.run();
        assert_eq!(h.state().settings.active_project, Some(second));
        assert!(h.state().sessions.iter().any(|s| s.busy));
        h.get_by_label("대화").click();
        h.run();
        h.get_by_label_contains("hold").click();
        h.run();
        assert_eq!(h.state().backend.as_ref().unwrap().pid(), first_pid);
        assert!(h.state().busy);
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
        app.backend =
            Some(Backend::start(&app.binary, crate::transport::test_cwd(), || {}).unwrap());
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
    fn project_switch_preserves_drafts_and_running_work() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
        let a = app.settings.active_project.unwrap();
        let b = app
            .settings
            .add_project("B", crate::transport::test_cwd(), "프로젝트 B 지침")
            .unwrap();
        app.prompt = "A의 작성 중 메시지".into();
        app.busy = true;
        app.switch_project(b);
        assert_eq!(app.settings.active_project, Some(b));
        assert_eq!(app.sessions.len(), 1);
        assert!(app.sessions[0].busy);
        assert!(app.prompt.is_empty());
        let mut params = json!({"cwd":crate::transport::test_cwd()});
        app.project_instructions(&mut params);
        assert_eq!(params["developerInstructions"], "프로젝트 B 지침");
        app.prompt = "B의 초안".into();
        app.switch_project(a);
        assert_eq!(app.prompt, "A의 작성 중 메시지");
        assert!(app.busy);
        app.switch_project(b);
        assert_eq!(app.prompt, "B의 초안");
        assert!(!app.ready && app.backend.is_none());
    }
    #[test]
    fn compact_settings_toggle_and_idle_settle() {
        use egui_kittest::{Harness, kittest::Queryable};
        for zoom in [1.0, 1.5] {
            let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
            let app = MiniApp::new(&cc, Settings::default(), "unused-test-backend".into());
            let mut harness = Harness::builder()
                .with_size(egui::vec2(560.0, 400.0))
                .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
            harness.ctx.set_style_of(
                egui::Theme::Dark,
                (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone(),
            );

            harness.ctx.set_zoom_factor(zoom);
            harness.run();
            assert!(harness.query_by_label("대화 없음").is_some());
            assert!(!harness.state().show_settings);
            // AccessKit bounds include the root DPI transform; egui input uses points.
            let point =
                harness.get_by_label("설정").rect().center() / harness.ctx.pixels_per_point();
            assert!(point.x < 560.0 / zoom && point.y < 400.0 / zoom);
            harness.hover_at(point);
            harness.run();
            harness.drag_at(point);
            harness.run();
            harness.drop_at(point);
            harness.run();
            assert!(harness.state().show_settings, "settings at zoom {zoom}");
            let labels =
                ["언어", "테마", "작업 폴더"].map(|label| harness.get_by_label(label).rect());
            assert!((labels[0].min.x - labels[1].min.x).abs() < 1.0);
            assert!((labels[1].min.x - labels[2].min.x).abs() < 1.0);
            let window = harness.get_by_label("환경 설정").rect();
            assert!(
                window.min.x >= 0.0 && window.max.x <= 561.0,
                "window bounds: {window:?}"
            );

            harness.key_press(egui::Key::Escape);
            harness.run();
            assert!(!harness.state().show_settings);
            assert!(harness.state().backend.is_none());
        }
    }

    #[test]
    fn usage_controls_fit_minimum_window_and_only_change_next_chat() {
        use egui_kittest::{Harness, kittest::Queryable};
        for zoom in [1.0, 1.5] {
            let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
            let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
            app.show_usage = true;
            app.rtk_path = Some("/tmp/rtk".into());
            let mut harness = Harness::builder()
                .with_size(egui::vec2(560.0, 400.0))
                .build_ui_state(|ui, app: &mut MiniApp| app.main_ui(ui), app);
            harness.ctx.set_style_of(
                egui::Theme::Dark,
                (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone(),
            );
            harness.ctx.set_zoom_factor(zoom);
            harness.run();
            let window = harness.get_by_label("사용량·절약").rect();
            assert!(window.min.x >= 0.0 && window.max.x <= 561.0, "{window:?}");
            assert!(window.min.y >= 0.0 && window.max.y <= 401.0, "{window:?}");
            if zoom == 1.0 {
                harness.get_by_label("Cargo 로그 필터 (RTK)").click();
                harness.run();
                assert!(harness.state().settings.active().unwrap().efficiency.rtk);
                assert!(!harness.state().active_efficiency.rtk);
            }
            harness.key_press(egui::Key::Escape);
            harness.run();
            assert!(!harness.state().show_usage);
        }
    }

    #[test]
    fn usage_notifications_reach_completed_record_without_double_counting() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
        app.thread_id = Some("thread".into());
        app.meter.attach("thread", true);
        app.meter.begin("project", "model", "medium", "medium");
        app.meter.turn("turn");
        let update = json!({"method":"thread/tokenUsage/updated","params":{
            "threadId":"thread","turnId":"turn","tokenUsage":{
                "total":{"inputTokens":2400,"cachedInputTokens":1600,"outputTokens":120,"reasoningOutputTokens":90,"totalTokens":2520},
                "last":{"inputTokens":2400,"outputTokens":120,"totalTokens":2520}}}});
        app.notification(update.clone());
        app.notification(update);
        app.notification(json!({"method":"turn/completed","params":{"threadId":"thread","turn":{"id":"turn","status":"completed"}}}));
        assert_eq!(app.meter.records.len(), 1);
        let tokens = app.meter.records[0].tokens.as_ref().unwrap();
        assert_eq!(tokens.total_tokens, 2520);
        assert_eq!(tokens.input_tokens, 2400);
        assert_eq!(tokens.output_tokens, 120);
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

    #[test]
    fn completion_race_failed_turn_and_approval_are_handled() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let mut app = MiniApp::new(&cc, Settings::default(), "codex".into());
        app.thread_id = Some("t1".into());
        app.busy = true;
        app.pending.insert(5, Pending::Turn);
        app.notification(json!({"method":"turn/completed","params":{
            "threadId":"t1","turn":{"id":"turn1","status":"interrupted"}}}));
        app.process(json!({"id":5,"result":{"turn":{"id":"turn1"}}}));
        assert!(!app.busy);
        assert!(app.turn_id.is_none());

        app.notification(json!({"method":"item/agentMessage/delta","params":{
            "threadId":"another-thread","itemId":"wrong","delta":"must not appear"}}));
        assert!(app.transcript.messages.is_empty());

        app.busy = true;
        app.submitted_prompt = "다시 시도할 작업".into();
        app.pending.insert(6, Pending::Turn);
        app.process(json!({"id":6,"error":{"code":-1,"message":"failed"}}));
        assert!(!app.busy);
        assert_eq!(app.prompt, "다시 시도할 작업");

        app.server_request(
            json!({"id":"approval-a","method":"item/commandExecution/requestApproval",
            "params":{"threadId":"t1","itemId":"cmd","command":"some command"}}),
        );
        assert_eq!(
            app.approvals.len(),
            1,
            "requests must wait for explicit UI action"
        );
        app.notification(json!({"method":"serverRequest/resolved","params":{"threadId":"t1","requestId":"approval-a"}}));
        assert!(app.approvals.is_empty());
    }
}
