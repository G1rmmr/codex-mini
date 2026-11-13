use crate::{
    state::{self, Project, Recent, Settings, Transcript, clipped, string},
    transport::{Backend, Event},
};
use eframe::egui::{self, Color32, RichText};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

const ACCENT: Color32 = Color32::from_rgb(194, 202, 213);
const CANVAS: Color32 = Color32::from_rgb(25, 26, 29);
const SURFACE: Color32 = Color32::from_rgb(34, 35, 39);
const BORDER: Color32 = Color32::from_rgb(61, 63, 69);
const MUTED: Color32 = Color32::from_rgb(190, 194, 203);

#[derive(Clone)]
enum Pending {
    Initialize,
    Account,
    Models,
    Start,
    Resume,
    History,
    Turn,
    Interrupt,
    Login,
    Ignore,
    Skills,
    SkillSaved,
    LocalSkills,
    PrepareThread { params: Value, resume: bool },
}

struct Approval {
    id: Value,
    method: String,
    params: Value,
    preview: String,
    preview_complete: bool,
    answers: HashMap<String, String>,
}

pub struct MiniApp {
    pub settings: Settings,
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
    diagnostics: VecDeque<String>,
    prompt: String,
    queued_prompt: Option<String>,
    submitted_prompt: String,
    status: String,
    error: Option<String>,
    account: String,
    auth_url: Option<String>,
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
}

impl MiniApp {
    pub fn new(cc: &eframe::CreationContext<'_>, mut settings: Settings, binary: String) -> Self {
        settings.prepare_projects();
        let ctx = &cc.egui_ctx;
        ctx.set_visuals(egui::Visuals::dark());
        let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals.panel_fill = CANVAS;
        style.visuals.window_fill = SURFACE;
        style.visuals.extreme_bg_color = CANVAS;
        style.visuals.selection.bg_fill = Color32::from_rgb(67, 77, 94);
        style.visuals.selection.stroke = egui::Stroke::new(1.0, Color32::WHITE);
        style.visuals.override_text_color = Some(Color32::from_rgb(239, 240, 243));
        style.visuals.weak_text_color = Some(MUTED);
        style.visuals.disabled_alpha = 0.72;
        style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        style.visuals.widgets.inactive.weak_bg_fill = SURFACE;
        style.visuals.widgets.inactive.bg_fill = SURFACE;
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(49, 51, 57);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(49, 51, 57);
        style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, MUTED);
        style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);
        style.spacing.item_spacing = egui::vec2(5.0, 4.0);
        style.spacing.button_padding = egui::vec2(6.0, 2.0);
        style.spacing.interact_size.y = 20.0;
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
        }
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(11.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(10.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
        style.animation_time = 0.0;
        style.visuals.text_cursor.blink = false;
        ctx.set_style_of(egui::Theme::Dark, style);
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
            meter: Default::default(),
            journal,
            usage_error: None,
            show_usage: false,
            active_efficiency: Default::default(),
            rtk_path: crate::efficiency::bundled_rtk(),
            local_skills: Vec::new(),
            mcp_draft: String::new(),
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
            diagnostics: VecDeque::new(),
            prompt: String::new(),
            queued_prompt: None,
            submitted_prompt: String::new(),
            status: "연결 전 · 대기 중".into(),
            error: None,
            account: String::new(),
            auth_url: None,
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
        self.settings.codex_binary = self.binary.clone();
        #[cfg(not(test))]
        if let Err(e) = self.settings.save() {
            self.error = Some(format!("설정 저장 실패: {e}"));
        }
    }

    fn connect(&mut self, ctx: &egui::Context) {
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
                self.backend = Some(backend);
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
        self.finish_usage("disconnected");
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
        self.status = "연결 종료 · Codex 프로세스 해제".into();
    }

    fn new_chat(&mut self) {
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

    fn submit(&mut self) {
        if !self.ready || self.busy || self.loading || self.prompt.trim().is_empty() {
            return;
        }
        if self.prompt.len() > 128 * 1024 {
            self.error = Some("메시지는 128 KiB 이하로 입력하세요.".into());
            return;
        }
        let prompt = self.prompt.clone();
        self.error = None;
        if self.thread_id.is_none() {
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
        if self.rpc(
            "turn/start",
            state::turn_params(
                &thread,
                &prompt,
                &self.settings.model,
                &self.effective_effort(),
            ),
            Pending::Turn,
        ) {
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
            self.prompt.clear();
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
        if let Some(project) = self.settings.active() {
            params["developerInstructions"] = project.instructions.clone().into();
        }
    }

    fn switch_project(&mut self, id: u64) {
        if self.busy || self.loading || self.settings.active_project == Some(id) {
            return;
        }
        let Some(project) = self.settings.projects.iter().find(|p| p.id == id) else {
            return;
        };
        let cwd = project.cwd.clone();
        if let Some(previous) = self.settings.active_project {
            self.drafts
                .insert(previous, std::mem::take(&mut self.prompt));
        }
        self.disconnect();
        self.transcript = Transcript::default();
        self.settings.active_project = Some(id);
        self.settings.cwd = cwd;
        self.prompt = self.drafts.remove(&id).unwrap_or_default();
        self.error = None;
        self.status = "프로젝트 전환 · 연결 전".into();
        self.save();
    }

    fn resume(&mut self, recent: Recent) {
        if recent.project_id != self.settings.active_project || self.busy || self.loading {
            return;
        }
        self.new_chat();
        self.active_efficiency = recent.efficiency.clone();
        let mut params = json!({"threadId":recent.id,"excludeTurns":true,
            "cwd":self.settings.cwd});
        self.project_instructions(&mut params);
        if self.prepare_thread(params, true) {
            self.loading = true;
            self.status = "이전 대화 불러오는 중…".into();
        }
    }

    fn projects_ui(&mut self, ui: &mut egui::Ui) {
        let can_change = !self.busy && !self.loading;
        ui.horizontal(|ui| {
            ui.strong("프로젝트");
            if ui
                .add_enabled(can_change, egui::Button::new("추가"))
                .clicked()
            {
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
                            can_change,
                            egui::Button::new(&project.name)
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
            .add_enabled(can_change, egui::Button::new("프로젝트 편집").frame(false))
            .clicked()
        {
            self.project_error = None;
            self.project_editor = self.settings.active().cloned();
        }
        ui.separator();
        if ui
            .add_enabled(self.ready && can_change, egui::Button::new("새 대화"))
            .clicked()
        {
            self.new_chat();
        }
        if !self.ready {
            ui.small("연결 후 대화를 열 수 있습니다.");
        }
        ui.small("최근 대화 · 최대 20개");
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .id_salt("project_chats")
            .show(ui, |ui| {
                for recent in self
                    .settings
                    .recent
                    .iter()
                    .filter(|r| r.project_id == self.settings.active_project)
                {
                    if ui
                        .add_enabled(
                            self.ready && can_change,
                            egui::Button::new(&recent.title)
                                .selected(self.thread_id.as_deref() == Some(&recent.id))
                                .frame(self.thread_id.as_deref() == Some(&recent.id))
                                .wrap_mode(egui::TextWrapMode::Truncate),
                        )
                        .on_hover_text(&recent.title)
                        .clicked()
                    {
                        chosen = Some(recent.clone());
                    }
                }
            });
        if let Some(recent) = chosen {
            self.resume(recent);
        }
    }

    fn project_editor_ui(&mut self, ctx: &egui::Context) {
        let Some(mut editor) = self.project_editor.take() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        egui::Window::new(if editor.id == 0 {
            "프로젝트 추가"
        } else {
            "프로젝트 편집"
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
            ui.label("이름");
            ui.add(
                egui::TextEdit::singleline(&mut editor.name)
                    .char_limit(120)
                    .desired_width(f32::INFINITY),
            );
            ui.label("작업 폴더");
            ui.add(
                egui::TextEdit::singleline(&mut editor.cwd)
                    .char_limit(4096)
                    .desired_width(f32::INFINITY),
            );
            ui.label("프로젝트 지침");
            ui.add(
                egui::TextEdit::multiline(&mut editor.instructions)
                    .char_limit(16 * 1024)
                    .desired_rows(5)
                    .desired_width(f32::INFINITY)
                    .hint_text("이 프로젝트에서 공통으로 지킬 작업 방식"),
            );
            ui.small("지침은 이 프로젝트의 새 대화·이어쓰기에 적용됩니다.");
            if let Some(error) = &self.project_error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            save = ui
                .add_enabled(!self.busy && !self.loading, egui::Button::new("저장"))
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
                    self.switch_project(id);
                } else {
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
                    if self.prompt.is_empty() {
                        self.prompt = self.submitted_prompt.clone();
                    }
                }
                _ => {}
            }
            self.status = "요청 실패 · 내용을 확인하세요".into();
            return;
        }
        let result = &message["result"];
        match pending {
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
                self.show_settings = false;
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
                if let Some(record) = self.meter.update(p) {
                    self.persist_usage(record);
                }
            }
            "item/started" | "item/completed" => {
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
            self.note("지원하지 않는 요청", &format!("{method}\n요청을 실행하지 않았습니다. 필요한 경우 공식 Codex CLI에서 이어가세요."));
            return;
        }
        let previous = self
            .transcript
            .messages
            .iter()
            .find(|m| m.id == string(&p, "itemId"))
            .map(|m| m.body.clone())
            .unwrap_or_default();
        let (preview, preview_complete) = approval_preview(&p, &previous);
        self.approvals.push_back(Approval {
            id,
            method,
            params: p,
            preview,
            preview_complete,
            answers: HashMap::new(),
        });
    }

    fn note(&mut self, title: &str, body: &str) {
        self.serial += 1;
        self.transcript
            .put(&format!("note-{}", self.serial), title, body, true, false);
    }

    fn approval_ui(&mut self, ctx: &egui::Context) {
        let mut response = None;
        if let Some(a) = self.approvals.front_mut() {
            egui::Window::new("확인이 필요합니다")
                .collapsible(false)
                .resizable(true)
                .default_width(520.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
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
                                                .hint_text("답변 직접 입력"),
                                        );
                                        ui.separator();
                                    }
                                }
                            } else {
                                ui.label("아래 작업을 검토한 뒤 결정하세요.");
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
                    if !a.preview_complete {
                        ui.colored_label(Color32::LIGHT_RED, "전체 내용을 표시할 수 없어 허용을 비활성화했습니다. CLI에서 검토하세요.");
                    }
                    ui.horizontal(|ui| {
                        if a.method == "item/tool/requestUserInput" {
                            let complete = a.params["questions"].as_array().is_some_and(|qs| {
                                qs.iter().all(|q| {
                                    a.answers
                                        .get(string(q, "id"))
                                        .is_some_and(|s| !s.trim().is_empty())
                                })
                            });
                            if ui
                                .add_enabled(complete, egui::Button::new("답변 보내기"))
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
                            if ui.add_enabled(a.preview_complete, egui::Button::new("요청한 권한 허용 · 이번 작업만")).clicked() {
                                response = Some(
                                    json!({"permissions":a.params["permissions"],"scope":"turn"}),
                                );
                            }
                            if ui.button("거절").clicked() {
                                response = Some(json!({"permissions":{},"scope":"turn"}));
                            }
                        } else {
                            let decisions = a.params["availableDecisions"].as_array();
                            let can_accept = decisions
                                .is_none_or(|ds| ds.iter().any(|v| v.as_str() == Some("accept")));
                            if ui
                                .add_enabled(can_accept && a.preview_complete, egui::Button::new("이번 한 번 허용"))
                                .clicked()
                            {
                                response = Some(json!({"decision":"accept"}));
                            }
                            let can_decline = decisions
                                .is_none_or(|ds| ds.iter().any(|v| v.as_str() == Some("decline")));
                            if ui
                                .add_enabled(can_decline, egui::Button::new("거절"))
                                .clicked()
                            {
                                response = Some(json!({"decision":"decline"}));
                            }
                        }
                        ui.small("작업 중단은 메인 창에서 할 수 있습니다.");
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
            Ok(config) => {
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
        if self.settings.effort.is_empty() {
            self.active_efficiency.effort.clone()
        } else {
            self.settings.effort.clone()
        }
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
        if !self.show_usage {
            return;
        }
        let mut open = true;
        egui::Window::new("사용량·절약")
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
        ui.strong("현재 작업");
        if let Some(record) = self.meter.current() {
            if let Some(tokens) = &record.tokens {
                token_grid(ui, "current_tokens", tokens);
            } else {
                ui.label("서버 사용량 수신 대기 중");
            }
            ui.label(format!(
                "{} · {} · 도구 {}회 · {}초",
                record.model,
                if record.effort.is_empty() {
                    "Codex 기본값"
                } else {
                    &record.effort
                },
                record.tools,
                record.elapsed_secs
            ));
            if record.partial {
                ui.weak("복원 후 관측분만 포함합니다. 첫 호출 이전 사용량은 제외됩니다.");
            }
        } else {
            ui.label("아직 측정한 작업이 없습니다.");
        }
        ui.weak("캐시는 입력에, 추론은 출력에 포함됩니다. 절감률·요금 추정값이 아닙니다.");
        if let Some(error) = &self.usage_error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        ui.separator();
        ui.strong("이 프로젝트 · 다음 새 대화부터");
        ui.weak("기존 대화는 시작할 때 선택한 설정을 유지합니다.");
        let Some(project_id) = self.settings.active_project else {
            return;
        };
        let Some(mut options) = self.settings.active().map(|p| p.efficiency.clone()) else {
            return;
        };
        let previous = options.clone();
        ui.horizontal(|ui| {
            ui.label("기본 추론");
            egui::ComboBox::from_id_salt("project_effort")
                .selected_text(if options.effort.is_empty() {
                    "Codex 기본값"
                } else {
                    &options.effort
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut options.effort, String::new(), "Codex 기본값");
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
        ui.weak("환경 설정에서 지정한 추론 값이 우선합니다. 모델별 지원 범위가 다릅니다.");
        ui.add_enabled(
            self.rtk_path.is_some() || options.rtk,
            egui::Checkbox::new(&mut options.rtk, "Cargo 로그 필터 (RTK)"),
        );
        ui.weak(if self.rtk_path.is_some() {
            "test / clippy / build / check만 필터링. 생략된 진단은 RTK 복구 힌트를 확인하세요."
        } else {
            "RTK 미설치 · scripts/install-rtk.sh 실행 후 앱을 다시 여세요."
        });
        ui.collapsing("스킬·MCP 제외", |ui| {
            ui.weak("필요 없는 항목만 선택하세요. 공유 Codex 설정은 바뀌지 않습니다.");
            if ui
                .add_enabled(self.ready, egui::Button::new("스킬 목록 불러오기"))
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
                            format!(
                                "제외: {}{}",
                                string(skill, "name"),
                                if globally_enabled {
                                    ""
                                } else {
                                    " (이미 꺼짐)"
                                }
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
            ui.label("MCP 제외 이름 · 쉼표 구분");
            ui.add(
                egui::TextEdit::singleline(&mut self.mcp_draft)
                    .desired_width(ui.available_width())
                    .hint_text("server_name, another_server"),
            );
            if ui.button("MCP 제외 적용").clicked() {
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
            if ui.button("제외 항목 초기화").clicked() {
                options.disabled_skills.clear();
                options.disabled_mcp.clear();
                self.mcp_draft.clear();
            }
            ui.label(format!(
                "스킬 {}개 · MCP {}개 제외",
                options.disabled_skills.len(),
                options.disabled_mcp.len()
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
            ui.strong(format!(
                "최근 작업 · {} / {}",
                self.meter.records.len(),
                crate::usage::KEEP
            ));
            if ui.button("JSON 복사").clicked() {
                ui.ctx().copy_text(
                    serde_json::to_string_pretty(&self.meter.records).unwrap_or_default(),
                );
            }
        });
        ui.weak("이 PC의 최근 200개 작업. 프롬프트·코드·응답 본문은 기록하지 않습니다.");
        for record in &self.meter.records {
            ui.push_id((&record.thread, &record.turn), |ui| {
                ui.collapsing(
                    format!(
                        "{} · {} · {} · {}초",
                        record.project, record.profile, record.status, record.elapsed_secs
                    ),
                    |ui| {
                        if let Some(tokens) = &record.tokens {
                            token_grid(ui, "record_tokens", tokens);
                        } else {
                            ui.label("서버 사용량 없음");
                        }
                        ui.label(format!("{} · 도구 {}회", record.model, record.tools));
                        if record.partial {
                            ui.weak("복원 후 관측분");
                        }
                    },
                );
            });
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let field_width = (ui.available_width() - 80.0).max(120.0);
        egui::Grid::new("settings_fields")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(64.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label("작업 폴더");
                    },
                );
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.settings.cwd).desired_width(field_width),
                );
                ui.end_row();
                ui.allocate_ui_with_layout(
                    egui::vec2(64.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label("모델");
                    },
                );
                ui.add_enabled_ui(!self.busy && !self.loading, |ui| {
                    egui::ComboBox::from_id_salt("model")
                        .selected_text(if self.settings.model.is_empty() {
                            "Codex 기본값"
                        } else {
                            &self.settings.model
                        })
                        .width(field_width)
                        .truncate()
                        .show_ui(ui, |ui| {
                            let mut changed = ui
                                .selectable_value(
                                    &mut self.settings.model,
                                    String::new(),
                                    "Codex 기본값",
                                )
                                .changed();
                            for model in &self.models {
                                let name = string(model, "model");
                                changed |= ui
                                    .selectable_value(
                                        &mut self.settings.model,
                                        name.to_owned(),
                                        name,
                                    )
                                    .changed();
                            }
                            if changed {
                                self.settings.effort.clear();
                            }
                        });
                });
                ui.end_row();
                ui.allocate_ui_with_layout(
                    egui::vec2(64.0, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label("추론");
                    },
                );
                ui.add_enabled_ui(!self.busy && !self.loading, |ui| {
                    let efforts = self
                        .models
                        .iter()
                        .find(|m| string(m, "model") == self.settings.model)
                        .and_then(|m| m["supportedReasoningEfforts"].as_array());
                    egui::ComboBox::from_id_salt("effort")
                        .selected_text(if self.settings.effort.is_empty() {
                            "기본값"
                        } else {
                            &self.settings.effort
                        })
                        .width(field_width)
                        .truncate()
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.settings.effort, String::new(), "기본값");
                            if let Some(efforts) = efforts {
                                for effort in efforts {
                                    let name = string(effort, "reasoningEffort");
                                    ui.selectable_value(
                                        &mut self.settings.effort,
                                        name.to_owned(),
                                        name,
                                    );
                                }
                            }
                        });
                });
                ui.end_row();
            });
        ui.add_space(8.0);
        ui.label(
            RichText::new("추가 권한이 필요하면 승인을 요청합니다.")
                .size(10.0)
                .color(MUTED),
        );
    }

    fn main_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let project = self
            .settings
            .active()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "프로젝트".into());
        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(CANVAS)
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Codex Mini").size(12.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new("설정")
                                    .frame(true)
                                    .selected(self.show_settings),
                            )
                            .clicked()
                        {
                            self.show_settings = !self.show_settings;
                        }
                        ui.toggle_value(&mut self.show_projects, "프로젝트");
                        ui.menu_button("대화", |ui| {
                            if ui
                                .add_enabled(
                                    self.ready && !self.busy && !self.loading,
                                    egui::Button::new("새 대화"),
                                )
                                .clicked()
                            {
                                self.new_chat();
                                ui.close();
                            }
                            ui.separator();
                            let mut chosen = None;
                            if self.settings.recent.is_empty() {
                                ui.weak("최근 대화 없음");
                            }
                            for recent in self
                                .settings
                                .recent
                                .iter()
                                .filter(|r| r.project_id == self.settings.active_project)
                            {
                                if ui
                                    .add_enabled(
                                        self.ready && !self.busy && !self.loading,
                                        egui::Button::new(&recent.title).frame(false),
                                    )
                                    .on_hover_text(&recent.cwd)
                                    .clicked()
                                {
                                    chosen = Some(recent.clone());
                                }
                            }
                            if let Some(recent) = chosen {
                                self.resume(recent);
                                ui.close();
                            }
                            if let Some(id) = &self.thread_id {
                                ui.separator();
                                if ui.button("대화 ID 복사").clicked() {
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
                .default_size(150.0)
                .size_range(130.0..=220.0)
                .frame(egui::Frame::new().fill(SURFACE).inner_margin(8.0))
                .show(ui, |ui| self.projects_ui(ui));
        }
        egui::Panel::bottom("status")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(CANVAS)
                    .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).size(10.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.backend.is_none() {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("연결").size(10.0))
                                        .min_size(egui::vec2(44.0, 20.0)),
                                )
                                .clicked()
                            {
                                self.connect(&ctx);
                            }
                        } else if ui
                            .add_enabled(
                                !self.busy,
                                egui::Button::new(RichText::new("연결 종료").size(10.0))
                                    .frame(false),
                            )
                            .clicked()
                        {
                            self.disconnect();
                        }
                    });
                });
            });
        egui::Panel::bottom("composer")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(CANVAS)
                    .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ui, |ui| {
                if let Some(error) = self.error.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(ui.visuals().error_fg_color, &error);
                        if ui.small_button("닫기").clicked() {
                            self.error = None;
                        }
                    });
                }
                egui::Frame::new()
                    .fill(SURFACE)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .corner_radius(8)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.prompt)
                                .char_limit(32 * 1024)
                                .desired_rows(2)
                                .desired_width(f32::INFINITY)
                                .frame(egui::Frame::NONE)
                                .hint_text(RichText::new("메시지 입력").color(MUTED))
                                .id_salt("prompt"),
                        );
                        ui.add_space(4.0);
                        let shortcut = ui
                            .input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Ctrl+Enter 전송").size(10.0).color(MUTED));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let enabled = self.ready
                                        && !self.busy
                                        && !self.loading
                                        && !self.prompt.trim().is_empty();
                                    let send = egui::Button::new(
                                        RichText::new("보내기").color(if enabled {
                                            CANVAS
                                        } else {
                                            MUTED
                                        }),
                                    )
                                    .fill(if enabled {
                                        Color32::from_rgb(221, 224, 230)
                                    } else {
                                        Color32::from_rgb(48, 50, 56)
                                    });
                                    if ui
                                        .add_enabled(enabled, send)
                                        .on_disabled_hover_text("연결 후 메시지를 입력하세요.")
                                        .clicked()
                                        || (enabled && shortcut)
                                    {
                                        self.submit();
                                    }
                                    if self.busy
                                        && ui
                                            .add_enabled(
                                                self.turn_id.is_some(),
                                                egui::Button::new("중단"),
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
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(CANVAS).inner_margin(egui::Margin::symmetric(10, 8)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(RichText::new(&project).size(13.0).strong()).truncate()).on_hover_text(&self.settings.cwd);

                });
                ui.add_space(12.0);
                egui::ScrollArea::vertical().id_salt("chat").stick_to_bottom(!self.transcript.messages.is_empty()).auto_shrink([false,false]).show(ui,|ui| {
                    if self.transcript.messages.is_empty() {
                        ui.add_space(16.0);
                        ui.label(RichText::new("대화 없음").size(12.0));
                        ui.add_space(2.0);
                        ui.label(RichText::new(if self.backend.is_none() { "연결 후 메시지를 입력하세요." } else { "아래에 작업 내용을 입력하세요." }).size(12.0).color(MUTED));
                    }
                    if self.transcript.trimmed { ui.weak("이전 내용 일부는 화면에서 생략했습니다. 원본은 Codex에 남아 있습니다."); }
                    for m in &self.transcript.messages {
                        ui.push_id(&m.id, |ui| {
                            if m.tool {
                                egui::CollapsingHeader::new(&m.title).show(ui,|ui| {
                                    ui.add(egui::Label::new(RichText::new(&m.body).monospace().size(10.0)).wrap().selectable(true));
                                    if ui.small_button("복사").clicked() { ctx.copy_text(m.body.clone()); }
                                });
                            } else {
                                egui::Frame::new().inner_margin(egui::Margin::symmetric(0, 6)).show(ui,|ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&m.title).size(10.0).color(MUTED));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                                            if ui.add(egui::Button::new(RichText::new("복사").size(10.0)).frame(false)).clicked() { ctx.copy_text(m.body.clone()); }
                                        });
                                    });
                                    ui.add(egui::Label::new(&m.body).wrap().selectable(true));
                                });
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
            egui::Window::new("프로젝트 목록")
                .open(&mut open)
                .default_width(220.0)
                .max_height((ctx.content_rect().height() - 70.0).max(100.0))
                .vscroll(true)
                .show(&ctx, |ui| self.projects_ui(ui));
            if !open {
                self.show_projects = false;
            }
        }
        self.project_editor_ui(&ctx);
        let global_was_open = self.global.open;
        if let Some(action) =
            self.global
                .ui(&ctx, self.ready, !self.busy && !self.loading, &self.binary)
        {
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
            egui::Window::new("환경 설정")
                .open(&mut open)
                .collapsible(false)
                .fixed_size(egui::vec2(
                    (ctx.content_rect().width() - 40.0).min(400.0),
                    (ctx.content_rect().height() - 80.0).min(270.0),
                ))
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .resizable(false)
                .vscroll(true)
                .show(&ctx, |ui| {
                    self.settings_ui(ui);
                    if ui.button("사용량·절약").clicked() {
                        self.show_usage = true;
                        self.show_settings = false;
                        self.mcp_draft = self
                            .settings
                            .active()
                            .map(|p| p.efficiency.disabled_mcp.join(", "))
                            .unwrap_or_default();
                    }
                    if ui.button("전역 설정 · 지침 / 스킬 / MCP").clicked() {
                        self.global.open = true;
                        self.show_settings = false;
                    }
                    if self.global.changed
                        && !self.busy
                        && !self.loading
                        && ui.button("변경 적용 · 다시 연결").clicked()
                    {
                        self.disconnect();
                        self.connect(&ctx);
                        self.global.changed = false;
                    }
                    ui.separator();
                    if !self.account.is_empty() {
                        ui.label(&self.account);
                    }
                    if self.ready
                        && self.account == "로그인 필요"
                        && ui.button("ChatGPT 로그인").clicked()
                    {
                        self.rpc(
                            "account/login/start",
                            json!({"type":"chatgpt"}),
                            Pending::Login,
                        );
                    }
                    ui.toggle_value(&mut self.show_diagnostics, "진단");
                });
            if !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.show_settings = false;
            }
        }
        if let Some(url) = &self.auth_url {
            egui::Window::new("로그인")
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.hyperlink_to("브라우저에서 ChatGPT 로그인 열기", url);
                });
        }
        self.usage_window(&ctx);
        self.approval_ui(&ctx);
        if self.show_diagnostics {
            egui::Window::new("진단")
                .open(&mut self.show_diagnostics)
                .default_width(600.0)
                .show(&ctx, |ui| {
                    ui.label(format!(
                        "GUI PID: {} · Codex PID: {}",
                        std::process::id(),
                        self.backend
                            .as_ref()
                            .map(|b| b.pid().to_string())
                            .unwrap_or_else(|| "연결 전".into())
                    ));
                    ui.label(format!("Codex 실행 파일: {}", self.binary));
                    ui.label("로그는 최대 40줄만 메모리에 보관합니다.");
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
    }
}

impl eframe::App for MiniApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(journal) = &self.journal {
            while let Ok(event) = journal.rx.try_recv() {
                match event {
                    crate::usage::DiskEvent::Loaded(records) => self.meter.merge(records),
                    crate::usage::DiskEvent::Error(error) => self.usage_error = Some(error),
                }
            }
        }
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
                    break;
                }
                None => return,
            }
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.main_ui(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save();
        self.disconnect();
    }
}

fn token_grid(ui: &mut egui::Ui, id: &str, tokens: &crate::usage::Tokens) {
    egui::Grid::new(id).num_columns(2).show(ui, |ui| {
        for (label, value) in [
            ("입력", tokens.input_tokens),
            ("캐시 입력 (포함)", tokens.cached_input_tokens),
            ("출력", tokens.output_tokens),
            ("추론 출력 (포함)", tokens.reasoning_output_tokens),
        ] {
            ui.label(label);
            ui.monospace(crate::usage::number(value));
            ui.end_row();
        }
    });
}

fn approval_preview(params: &Value, previous: &str) -> (String, bool) {
    let mut preview = String::new();
    for (key, label) in [
        ("command", "실행할 명령"),
        ("cwd", "작업 폴더"),
        ("reason", "요청 이유"),
        ("grantRoot", "허용할 경로"),
    ] {
        if let Some(value) = params[key].as_str().filter(|s| !s.is_empty()) {
            preview.push_str(&format!("{label}\n{value}\n\n"));
        }
    }
    for (key, label) in [
        ("networkApprovalContext", "네트워크 접근"),
        ("permissions", "요청한 권한"),
        ("additionalPermissions", "추가 권한"),
    ] {
        if let Some(value) = params.get(key).filter(|v| !v.is_null()) {
            preview.push_str(&format!(
                "{label}\n{}\n\n",
                serde_json::to_string_pretty(value).unwrap_or_default()
            ));
        }
    }
    if !previous.is_empty() {
        preview.push_str(&format!("작업 내용\n{previous}"));
    }
    let complete = preview.len() <= state::MAX_BODY
        && !previous.contains("[화면 표시 생략]")
        && !preview.is_empty();
    (clipped(&preview, state::MAX_BODY), complete)
}

#[cfg(not(test))]
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
mod tests {
    use super::*;

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
        editor.cwd = "/tmp".into();
        editor.instructions = "한국어로 답변".into();
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
            "한국어로 답변"
        );
    }
    #[test]
    fn project_switch_preserves_drafts_and_blocks_running_work() {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        let mut app = MiniApp::new(&cc, Settings::default(), "unused".into());
        let a = app.settings.active_project.unwrap();
        let b = app
            .settings
            .add_project("B", "/tmp", "프로젝트 B 지침")
            .unwrap();
        app.prompt = "A의 작성 중 메시지".into();
        app.busy = true;
        app.switch_project(b);
        assert_eq!(app.settings.active_project, Some(a));
        app.busy = false;
        app.switch_project(b);
        assert!(app.prompt.is_empty());
        let mut params = json!({"cwd":"/tmp"});
        app.project_instructions(&mut params);
        assert_eq!(params["developerInstructions"], "프로젝트 B 지침");
        app.prompt = "B의 초안".into();
        app.switch_project(a);
        assert_eq!(app.prompt, "A의 작성 중 메시지");
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
                ["작업 폴더", "모델", "추론"].map(|label| harness.get_by_label(label).rect());
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
    fn approval_preview_shows_scope_and_blocks_truncated_actions() {
        let (text, complete) = approval_preview(
            &json!({"command":"ls", "cwd":"/tmp", "threadId":"private-id", "additionalPermissions":{"network":true}}),
            "",
        );
        assert!(complete);
        assert!(text.contains("ls") && text.contains("/tmp") && text.contains("network"));
        assert!(!text.contains("private-id"));
        assert!(!approval_preview(&json!({"command":"x".repeat(state::MAX_BODY+1)}), "").1);
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
