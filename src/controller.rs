use crate::i18n::Language;
use crate::runtime::Wake;
use crate::{
    state::{self, Project, Recent, Settings, Transcript, clipped, string},
    transport::{Backend, Event},
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(super) struct QueuedJob {
    pub order: u64,
    pub prompt: String,
    pub model: String,
    pub effort: String,
}

#[derive(Clone)]
pub(super) enum Pending {
    Initialize,
    Delete(String),
    Compact,
    Account,
    RateLimits,
    Models,
    Start,
    Resume,
    History { older: bool },
    Turn,
    Steer { local_id: String, prompt: String },
    Interrupt,
    Login,
    CancelLogin,
    Ignore,
    Skills,
    SkillSaved,
    LocalSkills,
    PrepareThread { params: Value, resume: bool },
}

pub(super) struct Approval {
    pub(super) id: Value,
    pub(super) method: String,
    pub(super) params: Value,
    pub(super) preview: String,
    pub(super) preview_complete: bool,
    pub(super) previous: String,
    pub(super) preview_language: Language,
    pub(super) answers: HashMap<String, String>,
}

// Only conversation state is swapped. Shell services and saved preferences stay shared.
#[derive(Default)]
pub(super) struct Session {
    pub(super) project_id: Option<u64>,
    pub(super) cwd: String,
    pub(super) model: String,
    pub(super) models: Vec<Value>,
    pub(super) account: String,
    pub(super) rate_limits: crate::usage::RateLimits,
    pub(super) needs_auth: bool,
    pub(super) login_id: Option<String>,
    pub(super) effort: String,
    pub(super) backend: Option<Backend>,
    pub(super) pending: HashMap<u64, Pending>,
    pub(super) next_id: u64,
    pub(super) ready: bool,
    pub(super) busy: bool,
    pub(super) loading: bool,
    pub(super) thread_id: Option<String>,
    pub(super) turn_id: Option<String>,
    pub(super) transcript: Transcript,
    pub(super) approvals: VecDeque<Approval>,
    pub(super) auto_accept: bool,
    pub(super) diagnostics: VecDeque<String>,
    pub(super) prompt: String,
    pub(super) prompt_composing: bool,
    pub(super) queued_prompt: Option<String>,
    pub(super) submitted_prompt: String,
    pub(super) submitted_message_id: Option<String>,
    pub(super) status: String,
    pub(super) error: Option<String>,
    pub(super) auth_url: Option<String>,
    pub(super) resume_after_connect: Option<Recent>,
    pub(super) started: Option<Instant>,
    pub(super) active_efficiency: crate::efficiency::Options,
    pub(super) local_skills: Vec<Value>,
    pub(super) mcp_draft: String,
    pub(super) compacting: bool,
    pub(super) compact_due: bool,
    pub(super) completed_turns: VecDeque<String>,
    pub(super) history_cursor: Option<String>,
    pub(super) history_loading: bool,
    pub(super) queued_job: Option<QueuedJob>,
    pub(super) meter: crate::usage::Meter,
}

impl Session {
    fn swap(&mut self, app: &mut MiniApp) {
        std::mem::swap(&mut self.project_id, &mut app.settings.active_project);
        std::mem::swap(&mut self.cwd, &mut app.settings.cwd);
        std::mem::swap(&mut self.model, &mut app.settings.model);
        std::mem::swap(&mut self.models, &mut app.models);
        std::mem::swap(&mut self.account, &mut app.account);
        std::mem::swap(&mut self.rate_limits, &mut app.rate_limits);
        std::mem::swap(&mut self.needs_auth, &mut app.needs_auth);
        std::mem::swap(&mut self.login_id, &mut app.login_id);
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
        std::mem::swap(&mut self.history_cursor, &mut app.history_cursor);
        std::mem::swap(&mut self.history_loading, &mut app.history_loading);
        std::mem::swap(&mut self.queued_job, &mut app.queued_job);
        self.meter.swap_current(&mut app.meter);
    }
}

pub struct MiniApp {
    pub settings: Settings,
    pub(super) persist_settings: bool,
    pub(super) sessions: Vec<Session>,
    pub(super) background_processing: bool,
    pub(super) deferred_save: bool,
    pub(super) discarded_threads: Vec<String>,
    pub(super) binary: String,
    pub(super) backend: Option<Backend>,
    pub(super) pending: HashMap<u64, Pending>,
    pub(super) next_id: u64,
    pub(super) ready: bool,
    pub(super) busy: bool,
    pub(super) loading: bool,
    pub(super) thread_id: Option<String>,
    pub(super) turn_id: Option<String>,
    pub(super) transcript: Transcript,
    pub(super) models: Vec<Value>,
    pub(super) approvals: VecDeque<Approval>,
    pub(super) auto_accept: bool,
    pub(super) diagnostics: VecDeque<String>,
    pub(super) prompt: String,
    pub(super) prompt_composing: bool,
    pub(super) queued_prompt: Option<String>,
    pub(super) submitted_prompt: String,
    pub(super) submitted_message_id: Option<String>,
    pub(super) status: String,
    pub(super) error: Option<String>,
    pub(super) account: String,
    pub(super) rate_limits: crate::usage::RateLimits,
    pub(super) needs_auth: bool,
    pub(super) login_id: Option<String>,
    pub(super) auth_url: Option<String>,
    pub(super) resume_after_connect: Option<Recent>,
    pub(super) started: Option<Instant>,
    pub(super) serial: u64,
    pub(super) project_editor: Option<Project>,
    pub(super) project_error: Option<String>,
    pub(super) drafts: HashMap<u64, String>,
    pub(super) global: crate::global::GlobalUi,
    pub(super) ui_context: Wake,
    pub(super) history_cursor: Option<String>,
    pub(super) history_loading: bool,
    pub(super) queued_job: Option<QueuedJob>,
    pub(super) meter: crate::usage::Meter,
    pub(super) journal: Option<crate::usage::Journal>,
    pub(super) usage_error: Option<String>,
    pub(super) active_efficiency: crate::efficiency::Options,
    pub(super) rtk_path: Option<std::path::PathBuf>,
    pub(super) local_skills: Vec<Value>,
    pub(super) mcp_draft: String,
    pub(super) delete_chat: Option<Recent>,
    pub(super) compacting: bool,
    pub(super) compact_due: bool,
    pub(super) desktop: crate::desktop::Desktop,
    pub(super) hidden: bool,
    pub(super) exiting: bool,
    pub(super) show_exit_confirmation: bool,
    pub(super) completed_turns: VecDeque<String>,
}

impl MiniApp {
    pub(super) fn shutdown(&mut self) {
        self.save();
        self.disconnect();
        for mut session in std::mem::take(&mut self.sessions) {
            session.swap(self);
            self.disconnect();
            session.swap(self);
        }
    }

    pub(super) fn running_count(&self) -> usize {
        usize::from(self.busy || self.queued_prompt.is_some())
            + self
                .sessions
                .iter()
                .filter(|s| s.busy || s.queued_prompt.is_some())
                .count()
    }

    fn schedule_next(&mut self) {
        if self.background_processing
            || self.running_count() >= self.settings.max_concurrent_tasks.max(1)
        {
            return;
        }
        let active = self.queued_job.as_ref().map(|j| (j.order, None));
        let next = self
            .sessions
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.queued_job.as_ref().map(|j| (j.order, Some(i))))
            .chain(active)
            .min_by_key(|(order, _)| *order);
        let Some((_, index)) = next else {
            return;
        };
        if let Some(index) = index {
            let mut session = self.sessions.remove(index);
            session.swap(self);
            self.run_queued();
            session.swap(self);
            self.sessions.insert(index, session);
        } else {
            self.run_queued();
        }
    }

    fn run_queued(&mut self) {
        if self.busy || self.loading || self.compacting || !self.approvals.is_empty() {
            return;
        }
        if !self.ready {
            if self.error.is_some() {
                return;
            }
            self.connect(&self.ui_context.clone());
            return;
        }
        let Some(job) = self.queued_job.take() else {
            return;
        };
        let draft = std::mem::replace(&mut self.prompt, job.prompt);
        self.settings.model = job.model;
        self.settings.effort = job.effort;
        self.submit();
        // Keep a failed job's text recoverable instead of losing a newer draft.
        if !self.busy
            && !self.loading
            && self.queued_prompt.is_none()
            && !self.prompt.is_empty()
            && !draft.is_empty()
        {
            let failed = self.prompt.clone();
            self.note("전송 실패", &failed);
        }
        if !draft.is_empty() {
            self.prompt = draft;
        }
    }

    pub(super) fn older_history(&mut self) {
        if self.busy || self.loading || self.history_loading {
            return;
        }
        if let (Some(thread), Some(cursor)) = (self.thread_id.clone(), self.history_cursor.clone())
        {
            self.history_loading = self.rpc("thread/turns/list", json!({"threadId":thread,"cursor":cursor,"limit":20,"itemsView":"full","sortDirection":"desc"}), Pending::History { older: true });
        }
    }

    pub(super) fn respond(&mut self, result: Value) {
        if let Some(a) = self.approvals.front() {
            let id = a.id.clone();
            if self.send(json!({"id":id,"result":result})) {
                self.approvals.pop_front();
            }
        }
    }
    pub fn new(ctx: &Wake, settings: Settings, binary: String) -> Self {
        Self::new_with_services(ctx, settings, binary, true)
    }

    pub fn new_with_services(
        ctx: &Wake,
        mut settings: Settings,
        binary: String,
        services: bool,
    ) -> Self {
        settings.max_concurrent_tasks = settings.max_concurrent_tasks.clamp(1, 8);
        settings.prepare_projects();
        #[cfg(not(test))]
        let journal = Settings::path()
            .filter(|_| services)
            .and_then(|p| p.parent().map(|p| p.join("usage")))
            .map(|path| {
                let ctx = ctx.clone();
                crate::usage::Journal::start(path, move || ctx.request_repaint())
            });
        #[cfg(test)]
        let journal = None;
        Self {
            persist_settings: services,
            sessions: Vec::new(),
            background_processing: false,
            deferred_save: false,
            discarded_threads: Vec::new(),
            desktop: if services {
                crate::desktop::Desktop::new(ctx, settings.language)
            } else {
                crate::desktop::Desktop::empty(ctx, settings.language)
            },
            hidden: false,
            exiting: false,
            show_exit_confirmation: false,
            completed_turns: VecDeque::new(),
            history_cursor: None,
            history_loading: false,
            queued_job: None,
            meter: Default::default(),
            journal,
            usage_error: None,
            active_efficiency: Default::default(),
            rtk_path: crate::efficiency::bundled_rtk(),
            local_skills: Vec::new(),
            mcp_draft: String::new(),
            delete_chat: None,
            compacting: false,
            compact_due: false,
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
            rate_limits: Default::default(),
            needs_auth: false,
            login_id: None,
            auth_url: None,
            resume_after_connect: None,
            started: None,
            serial: 0,
            project_editor: None,
            project_error: None,
            drafts: HashMap::new(),
            global: crate::global::GlobalUi::default(),
            ui_context: ctx.clone(),
        }
    }

    pub(super) fn send(&mut self, value: Value) -> bool {
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

    pub(super) fn rpc(&mut self, method: &str, params: Value, pending: Pending) -> bool {
        let id = self.next_id;
        self.next_id += 1;
        if self.send(json!({"id":id,"method":method,"params":params})) {
            self.pending.insert(id, pending);
            true
        } else {
            false
        }
    }

    pub(super) fn save(&mut self) {
        if !self.persist_settings {
            return;
        }
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

    pub(super) fn refresh_rate_limits(&mut self) {
        if self.ready
            && !self.needs_auth
            && !self
                .pending
                .values()
                .any(|p| matches!(p, Pending::RateLimits))
        {
            self.rate_limits.error = None;
            self.rpc("account/rateLimits/read", json!({}), Pending::RateLimits);
        }
    }

    pub(super) fn connect(&mut self, ctx: &Wake) {
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

    pub(super) fn disconnect(&mut self) {
        if let Some(job) = self.queued_job.take() {
            if self.prompt.is_empty() {
                self.prompt = job.prompt;
            } else {
                self.prompt.push_str(&format!("\n\n{}", job.prompt));
            }
        }
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
        self.rate_limits = Default::default();
        self.needs_auth = false;
        self.login_id = None;
        self.auth_url = None;
        self.resume_after_connect = None;
        self.status = "연결 종료 · Codex 프로세스 해제".into();
    }

    pub(super) fn has_conversation(&self) -> bool {
        self.thread_id.is_some()
            || self.busy
            || self.loading
            || !self.approvals.is_empty()
            || self.queued_job.is_some()
            || !self.prompt.is_empty()
    }

    pub(super) fn suspend_idle_connection(&mut self) {
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
        self.rate_limits = Default::default();
        self.needs_auth = false;
        self.login_id = None;
        self.auth_url = None;
        self.status = "연결 종료 · Codex 프로세스 해제".into();
    }

    pub(super) fn park_conversation(&mut self) -> bool {
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

    pub(super) fn activate_session(&mut self, index: usize) {
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

    pub(super) fn new_chat(&mut self) {
        if self.busy || self.loading || self.queued_job.is_some() || !self.approvals.is_empty() {
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
        self.history_cursor = None;
        self.history_loading = false;
        self.active_efficiency = Default::default();
        self.meter.attach("", false);
        self.approvals.clear();
        self.turn_id = None;
        self.error = None;
        self.status = "새 대화".into();
    }

    pub(super) fn can_send(&self) -> bool {
        self.ready
            && !self.needs_auth
            && !self.loading
            && !self.compacting
            && (!self.busy || (self.thread_id.is_some() && self.turn_id.is_some()))
    }

    pub(super) fn user_message_id(&mut self) -> String {
        self.serial += 1;
        format!("user-{}-{}", std::process::id(), self.serial)
    }

    pub(super) fn submit(&mut self) {
        if !self.can_send() || self.prompt_composing || self.prompt.trim().is_empty() {
            return;
        }
        if self.prompt.len() > 128 * 1024 {
            self.error = Some("메시지는 128 KiB 이하로 입력하세요.".into());
            return;
        }
        if self.queued_job.is_some() {
            return;
        }
        if !self.busy && self.running_count() >= self.settings.max_concurrent_tasks {
            self.serial += 1;
            self.queued_job = Some(QueuedJob {
                order: self.serial,
                prompt: std::mem::take(&mut self.prompt),
                model: self.settings.model.clone(),
                effort: self.settings.effort.clone(),
            });
            self.status = "대기열에 추가됨".into();
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

    pub(super) fn start_turn(&mut self, prompt: String) {
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

    pub(super) fn project_instructions(&self, params: &mut Value) {
        if let Some(project) = self
            .settings
            .active()
            .filter(|p| !p.instructions.trim().is_empty())
        {
            params["developerInstructions"] = project.instructions.clone().into();
        }
    }

    pub(super) fn switch_project(&mut self, id: u64) {
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

    pub(super) fn resume(&mut self, recent: Recent) {
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

    pub(super) fn compact(&mut self) {
        if !self.ready || self.busy || self.loading {
            return;
        }
        let Some(id) = self.thread_id.clone() else {
            return;
        };
        if self.running_count() >= self.settings.max_concurrent_tasks {
            self.compact_due = true;
            self.status = "압축 대기".into();
            return;
        }
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

    pub(super) fn move_chat(&mut self, id: &str, project_id: u64) {
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

    pub(super) fn process(&mut self, message: Value) {
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
            if matches!(pending, Pending::RateLimits) {
                self.rate_limits.error = Some(clipped(&error.to_string(), 500));
                return;
            }
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
                | Pending::History { .. }
                | Pending::PrepareThread { .. } => {
                    self.loading = false;
                    self.queued_prompt = None;
                    self.history_loading = false;
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
                self.rpc(
                    "account/read",
                    json!({"refreshToken":false}),
                    Pending::Account,
                );
                self.rpc("model/list", json!({"limit":50}), Pending::Models);
            }
            Pending::Account => {
                let account = &result["account"];
                self.needs_auth =
                    account.is_null() && result["requiresOpenaiAuth"].as_bool() == Some(true);
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
                if matches!(
                    account["type"].as_str(),
                    Some("chatgpt" | "chatgptAuthTokens")
                ) {
                    self.refresh_rate_limits();
                }
            }
            Pending::RateLimits => self.rate_limits.update(result, true),
            Pending::Models => {
                if let Some(data) = result["data"].as_array() {
                    for model in data {
                        if !self.models.iter().any(|m| m["model"] == model["model"]) {
                            self.models.push(model.clone());
                        }
                    }
                }
                if let Some(cursor) = result["nextCursor"].as_str() {
                    self.rpc(
                        "model/list",
                        json!({"limit":50,"cursor":cursor}),
                        Pending::Models,
                    );
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
                    self.loading = self.rpc("thread/turns/list",json!({"threadId":thread,"limit":20,"itemsView":"full","sortDirection":"desc"}),Pending::History { older: false });
                } else {
                    self.transcript = Transcript::default();
                    if let Some(prompt) = self.queued_prompt.take() {
                        self.start_turn(prompt);
                    }
                }
            }
            Pending::History { older } => {
                self.loading = false;
                self.history_loading = false;
                self.history_cursor = result["nextCursor"].as_str().map(str::to_owned);
                let mut page = Transcript::default();
                if let Some(turns) = result["data"].as_array() {
                    for turn in turns.iter().rev() {
                        if let Some(items) = turn["items"].as_array() {
                            for item in items {
                                page.item(item);
                            }
                        }
                    }
                }
                if older {
                    let mut current = std::mem::take(&mut self.transcript.messages);
                    page.messages
                        .retain(|m| !current.iter().any(|c| c.id == m.id));
                    page.messages.append(&mut current);
                    // Older pages evict the newest visible items, never the page just requested.
                    while page.messages.len() > 160
                        || page.messages.iter().map(|m| m.body.len()).sum::<usize>()
                            > 2 * 1024 * 1024
                    {
                        page.messages.pop_back();
                        page.trimmed = true;
                    }
                }
                self.transcript = page;
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
            Pending::Login => {
                self.auth_url = result["authUrl"]
                    .as_str()
                    .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
                    .map(str::to_owned);
                self.login_id = result["loginId"].as_str().map(str::to_owned);
                if self.auth_url.is_none() {
                    self.error = Some("로그인 주소를 받지 못했습니다. 다시 시도하세요.".into());
                }
            }
            Pending::CancelLogin => {
                self.auth_url = None;
                self.login_id = None;
                self.error = None;
            }
            Pending::Ignore => {}
        }
    }

    pub(super) fn notification(&mut self, message: Value) {
        let method = string(&message, "method");
        let p = &message["params"];
        if let Some(thread) = p["threadId"].as_str()
            && self.thread_id.as_deref() != Some(thread)
        {
            return;
        }
        match method {
            "account/rateLimits/updated" => self.rate_limits.update(p, false),
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
                self.rate_limits = Default::default();
                self.auth_url = None;
                self.login_id = None;
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

    pub(super) fn server_request(&mut self, message: Value) {
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

    pub(super) fn accept_pending_automatically(&mut self) {
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

    pub(super) fn note(&mut self, title: &str, body: &str) {
        self.serial += 1;
        self.transcript
            .put(&format!("note-{}", self.serial), title, body, true, false);
    }

    pub(super) fn prepare_thread(&mut self, params: Value, resume: bool) -> bool {
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

    pub(super) fn start_thread(
        &mut self,
        mut params: Value,
        resume: bool,
        inherited: &Value,
    ) -> bool {
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

    pub(super) fn effective_effort(&self) -> String {
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

    pub(super) fn persist_usage(&mut self, record: crate::usage::Record) {
        if let Some(journal) = &self.journal
            && let Err(error) = journal.save(record)
        {
            self.usage_error = Some(error);
        }
    }

    pub(super) fn finish_usage(&mut self, status: &str) {
        if let Some(record) = self.meter.finish(status) {
            self.persist_usage(record);
        }
    }

    pub(super) fn drain_backend(&mut self, ctx: &Wake) {
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

    pub(super) fn tick_logic(&mut self, ctx: &Wake) {
        for approval in &mut self.approvals {
            if approval.preview_language != self.settings.language {
                (approval.preview, approval.preview_complete) =
                    approval_preview(&approval.params, &approval.previous, self.settings.language);
                approval.preview_language = self.settings.language;
            }
        }

        if let Some(journal) = &self.journal {
            while let Ok(event) = journal.rx.try_recv() {
                match event {
                    crate::usage::DiskEvent::Loaded(records) => self.meter.merge(records),
                    crate::usage::DiskEvent::Error(error) => self.usage_error = Some(error),
                }
            }
        }
        self.drain_backend(ctx);
        let mut sessions = std::mem::take(&mut self.sessions);
        self.background_processing = true;
        for session in &mut sessions {
            session.swap(self);
            self.drain_backend(ctx);
            // Cached conversations retain text and drafts, not idle CLI processes.
            self.suspend_idle_connection();
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
        self.global.poll();
        if self.running_count() < self.settings.max_concurrent_tasks {
            if self.compact_due && self.ready && !self.busy && !self.loading {
                self.compact();
            } else if let Some(index) = self
                .sessions
                .iter()
                .position(|s| s.compact_due && s.ready && !s.busy && !s.loading)
            {
                let mut session = self.sessions.remove(index);
                session.swap(self);
                self.compact();
                session.swap(self);
                self.sessions.insert(index, session);
            }
        }
        self.schedule_next();
        self.desktop.update(self.settings.language, &self.status);
    }
}

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

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
