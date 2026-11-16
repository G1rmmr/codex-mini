//! Iced front end. Protocol workers wake the UI; a hidden/idle window has no timer.
use crate::{
    controller::{MiniApp, Pending},
    i18n::Language,
    preferences::Theme,
    runtime::Wake,
    state::{self, Project, Settings, string},
};
use iced::{
    Element,
    Length::{self, Fill},
    Subscription, Task,
    widget::{
        self, button, checkbox, column, container, markdown, pick_list, row, scrollable, space,
        text, text_editor, text_input,
    },
    window,
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Chat,
    Settings,
    Project,
    Global,
    Usage,
    Resources,
}
#[derive(Debug, Clone)]
pub enum Message {
    Menu(Popup),
    Dismiss,
    Capture,
    Captured(window::Screenshot),
    Wake,
    Window(window::Id, window::Event),
    Opened(window::Id),
    Scrolled(bool),
    Login,
    CancelLogin,
    Composing(bool),
    Clock,
    Page(Page),
    Sidebar,
    Connect,
    Disconnect,
    NewChat,
    SelectProject(u64),
    Resume(String),
    Activate(usize),
    Compose(text_editor::Action),
    Send,
    Stop,
    Model(String),
    Effort(String),
    AutoAccept(bool),
    Expand(String),
    Raw(String),
    SelectText(String, text_editor::Action),
    Copy(String),
    Link(String),
    LinkOpened(Result<(), String>),
    Older,
    MoreVisible,
    Latest,
    ProjectNew,
    ProjectEdit,
    ProjectField(u8, String),
    ProjectInstructions(text_editor::Action),
    ProjectSave,
    Language(bool),
    Light(bool),
    Notifications(bool),
    Concurrency(usize),
    GlobalTab(usize),
    SkillFilter(String),
    McpArguments(text_editor::Action),
    GlobalEdit(text_editor::Action),
    GlobalField(String, String),
    GlobalSave(bool),
    GlobalReload,
    Binary(String),
    SaveBinary,
    RefreshSkills,
    Skill(String, bool),
    LocalSkill(String, bool),
    McpField(u8, String),
    McpHttp(bool),
    McpApply,
    McpEdit(String),
    McpToggle(String, bool),
    LocalMcp(String, bool),
    EditMcpExclusions(text_editor::Action),
    SaveMcpExclusions,
    ProjectEffort(String),
    Rtk(bool),
    Answer(String, String),
    Approve(bool),
    AnswerSend,
    CopyUsage,
    RefreshUsage,
    ChatOptions(String),
    MoveChat(String, u64),
    DeleteChat(String),
    ConfirmDelete,
    CancelDelete,
    Compact,
    CancelQueued(Option<usize>),
    Hide,
    Quit,
    ConfirmQuit,
    CancelQuit,
    RefreshResources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    Model,
    Chat,
    Composer,
    Usage,
}

struct CachedMarkdown {
    source: String,
    parsed: markdown::Content,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Choice {
    value: String,
    label: String,
}
impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
fn choices<'a>(
    values: Vec<String>,
    selected: &str,
    default: &str,
    action: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let options: Vec<_> = values
        .into_iter()
        .map(|value| Choice {
            label: if value.is_empty() {
                default.into()
            } else {
                value.clone()
            },
            value,
        })
        .collect();
    let selection = options
        .iter()
        .find(|c| c.value == selected)
        .cloned()
        .unwrap_or_else(|| Choice {
            value: selected.into(),
            label: selected.into(),
        });
    pick_list(options, Some(selection), move |choice: Choice| {
        action(choice.value)
    })
    .text_size(14)
    .width(Fill)
    .into()
}

pub struct App {
    popup: Option<Popup>,
    core: MiniApp,
    wake: Wake,
    window: window::Id,
    page: Page,
    sidebar: bool,
    width: f32,
    height: f32,
    composer: text_editor::Content,
    instructions: text_editor::Content,
    global_editor: text_editor::Content,
    mcp_arguments: text_editor::Content,
    mcp_exclusions: text_editor::Content,
    exclusions_source: String,
    global_source: String,
    expanded: HashSet<String>,
    raw: HashMap<String, text_editor::Content>,
    markdown: HashMap<String, CachedMarkdown>,
    visible: usize,
    follow_bottom: bool,
    chat_options: Option<String>,
    monitor: Option<crate::resources::Monitor>,
    // One stable ID preserves scrolling between streaming updates.
    scroll: widget::Id,
    composer_id: widget::Id,
    smoke_output: Option<std::path::PathBuf>,
    benchmark_output: Option<std::path::PathBuf>,
    booted: std::time::Instant,
}

pub fn run(
    settings: Settings,
    binary: String,
    smoke_output: Option<std::path::PathBuf>,
    benchmark_output: Option<std::path::PathBuf>,
) -> iced::Result {
    crate::typography::initialize();
    let booted = std::time::Instant::now();
    let boot = move || {
        let wake = Wake::default();
        let mut core = if smoke_output.is_some() {
            MiniApp::new_with_services(&wake, settings.clone(), binary.clone(), false)
        } else {
            MiniApp::new(&wake, settings.clone(), binary.clone())
        };
        if smoke_output.is_some() {
            populate_preview(&mut core);
            if std::env::args().any(|a| a == "--approval") {
                core.server_request(json!({"id":"preview-approval","method":"item/commandExecution/requestApproval","params":{"command":"cargo test --locked","reason":"프로젝트 테스트 실행","availableDecisions":["accept","decline"]}}));
            }
        }
        let (id, open) = window::open(window::Settings {
            size: if std::env::args().any(|a| a == "--compact") {
                iced::Size::new(560.0, 440.0)
            } else {
                iced::Size::new(940.0, 700.0)
            },
            min_size: Some(iced::Size::new(560.0, 440.0)),
            position: window::Position::Centered,
            exit_on_close_request: false,
            icon: window::icon::from_rgba(
                include_bytes!("../assets/codex-mini-64.rgba").to_vec(),
                64,
                64,
            )
            .ok(),
            ..Default::default()
        });
        let mut app = App {
            popup: std::env::args()
                .any(|a| a == "--usage-popup")
                .then_some(Popup::Usage),
            core,
            wake,
            window: id,
            page: if std::env::args().any(|a| a == "--usage") {
                Page::Usage
            } else if std::env::args().any(|a| a == "--settings") {
                Page::Settings
            } else {
                Page::Chat
            },
            sidebar: !std::env::args().any(|a| a == "--compact"),
            width: 940.0,
            height: 700.0,
            composer: text_editor::Content::new(),
            instructions: text_editor::Content::new(),
            global_editor: text_editor::Content::new(),
            mcp_arguments: text_editor::Content::new(),
            mcp_exclusions: text_editor::Content::new(),
            exclusions_source: String::new(),
            global_source: String::new(),
            expanded: HashSet::new(),
            raw: HashMap::new(),
            markdown: HashMap::new(),
            visible: 40,
            follow_bottom: true,
            chat_options: None,
            monitor: None,
            scroll: widget::Id::unique(),
            composer_id: widget::Id::unique(),
            smoke_output: smoke_output.clone(),
            benchmark_output: benchmark_output.clone(),
            booted,
        };
        app.sync();
        (app, open.map(Message::Opened))
    };
    let daemon = iced::daemon(boot, App::update, App::view)
        .executor::<crate::runtime::Executor>()
        .title(|_: &App, _| "Codex Mini".to_owned())
        .theme(|app: &App, _| app.core.settings.theme.iced())
        .subscription(App::subscription)
        .settings(iced::Settings {
            default_text_size: iced::Pixels(14.0),
            antialiasing: false,
            ..Default::default()
        });
    daemon.run()
}

impl App {
    fn label<'a>(&self, ko: &'a str, en: &'a str) -> &'a str {
        if self.core.settings.language == Language::Korean {
            ko
        } else {
            en
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![self.wake.subscription().map(|_| Message::Wake)];
        subscriptions.push(iced::event::listen_raw(|event, _, id| match event {
            iced::Event::Window(
                event @ (window::Event::CloseRequested
                | window::Event::Resized(_)
                | window::Event::Focused),
            ) => Some(Message::Window(id, event)),
            iced::Event::InputMethod(iced::advanced::input_method::Event::Preedit(content, _)) => {
                Some(Message::Composing(!content.is_empty()))
            }
            iced::Event::InputMethod(
                iced::advanced::input_method::Event::Commit(_)
                | iced::advanced::input_method::Event::Closed,
            ) => Some(Message::Composing(false)),
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                ..
            }) => Some(Message::Dismiss),
            _ => None,
        }));
        if self.page == Page::Resources && !self.core.hidden {
            subscriptions
                .push(iced::time::every(Duration::from_secs(2)).map(|_| Message::RefreshResources));
        }
        let usage_visible = self.page == Page::Usage || self.popup == Some(Popup::Usage);
        if !self.core.hidden
            && ((self.core.busy && self.page == Page::Chat)
                || (self.core.running_count() > 0 && usage_visible))
        {
            subscriptions.push(iced::time::every(Duration::from_secs(1)).map(|_| Message::Clock));
        } else if usage_visible && !self.core.hidden {
            subscriptions.push(iced::time::every(Duration::from_secs(60)).map(|_| Message::Clock));
        }
        Subscription::batch(subscriptions)
    }

    fn sync(&mut self) {
        if let Some(monitor) = &mut self.monitor {
            monitor.poll();
        }
        if self.core.hidden {
            self.markdown.clear();
            return;
        }
        if self.page == Page::Global && self.mcp_arguments.text() != self.core.global.mcp_args {
            self.mcp_arguments = text_editor::Content::with_text(&self.core.global.mcp_args);
        }
        if self.page == Page::Settings {
            let source = self
                .core
                .settings
                .active()
                .map(|p| p.efficiency.disabled_mcp.join("\n"))
                .unwrap_or_default();
            if source != self.exclusions_source {
                self.mcp_exclusions = text_editor::Content::with_text(&source);
                self.exclusions_source = source;
            }
        }
        if self.composer.text() != self.core.prompt {
            self.composer = text_editor::Content::with_text(&self.core.prompt);
        }
        if self.page == Page::Global {
            let source = self.core.global.docs.as_ref().map(|d| {
                if self.core.global.tab == 0 {
                    &d.agents.text
                } else {
                    &d.config.text
                }
            });
            if let Some(source) = source.filter(|s| *s != &self.global_source) {
                self.global_source = source.clone();
                self.global_editor = text_editor::Content::with_text(source);
            }
        }
        let prefix = self.core.thread_id.as_deref().unwrap_or("new");
        let mut retained = HashSet::new();
        for m in self.core.transcript.messages.iter().skip(
            self.core
                .transcript
                .messages
                .len()
                .saturating_sub(self.visible),
        ) {
            if m.tool || m.title == "나" {
                continue;
            }
            let key = format!("{prefix}/{}", m.id);
            retained.insert(key.clone());
            let cache = self.markdown.entry(key).or_insert_with(|| CachedMarkdown {
                source: String::new(),
                parsed: markdown::Content::new(),
            });
            if cache.source != m.body {
                if m.body.starts_with(&cache.source) {
                    cache.parsed.push_str(&m.body[cache.source.len()..]);
                } else {
                    cache.parsed = markdown::Content::new();
                    cache.parsed.push_str(&m.body);
                }
                cache.source.clone_from(&m.body);
            }
        }
        self.markdown.retain(|k, _| retained.contains(k));
        // Expanded IDs never accumulate across conversations.
        self.expanded
            .retain(|k| self.core.transcript.messages.iter().any(|m| k == &m.id));
        self.raw
            .retain(|k, _| self.core.transcript.messages.iter().any(|m| k == &m.id));
        for (id, editor) in &mut self.raw {
            if let Some(message) = self.core.transcript.messages.iter().find(|m| &m.id == id)
                && editor.text() != message.body
            {
                *editor = text_editor::Content::with_text(&message.body);
            }
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let mut task = Task::none();
        match message {
            Message::Dismiss => {
                if self.popup.take().is_none() {
                    if self.page != Page::Chat {
                        self.page = Page::Chat;
                    } else if self.width < 760. {
                        self.sidebar = false;
                    }
                }
            }
            Message::Menu(popup) => {
                self.popup = if self.popup == Some(popup) {
                    None
                } else {
                    Some(popup)
                };
                if self.popup == Some(Popup::Usage) {
                    self.core.refresh_rate_limits();
                }
            }
            Message::Capture => task = window::screenshot(self.window).map(Message::Captured),
            Message::Captured(screenshot) => {
                if let Some(path) = &self.smoke_output {
                    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                        let file = std::fs::File::create(path)?;
                        let mut encoder =
                            png::Encoder::new(file, screenshot.size.width, screenshot.size.height);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        let mut writer = encoder.write_header()?;
                        writer.write_image_data(&screenshot.rgba)?;
                        Ok(())
                    })();
                    if let Err(error) = result {
                        eprintln!("UI screenshot failed: {error}");
                        std::process::exit(1);
                    }
                    task = iced::exit();
                }
            }
            Message::Wake => {
                let old_auth_url = self.core.auth_url.clone();
                let was_connected = self.core.ready && !self.core.needs_auth;
                self.core.tick_logic(&self.wake);
                let mut tasks = Vec::new();
                if !was_connected && self.core.ready && !self.core.needs_auth {
                    tasks.push(widget::operation::focus(self.composer_id.clone()));
                }
                if self.core.auth_url != old_auth_url
                    && let Some(url) = &self.core.auth_url
                {
                    tasks.push(Task::done(Message::Link(url.clone())));
                }
                while let Some(action) = self.core.desktop.next_action() {
                    tasks.push(match action {
                        crate::desktop::Action::Show => self.show(),
                        crate::desktop::Action::Hide => self.hide(),
                        crate::desktop::Action::Quit => self.quit(),
                        crate::desktop::Action::Problem(error) => {
                            self.core.desktop.last_error = Some(error);
                            Task::none()
                        }
                    });
                }
                task = Task::batch(tasks);
            }
            Message::Opened(id) => {
                self.window = id;
                task = widget::operation::focus(self.composer_id.clone());
                if self.smoke_output.is_none() {
                    self.core.connect(&self.wake);
                }
                if self.smoke_output.is_some() && !std::env::args().any(|a| a == "--interactive") {
                    task = task.chain(Task::perform(
                        async {
                            tokio::time::sleep(Duration::from_millis(800)).await;
                        },
                        |_| Message::Capture,
                    ));
                }
            }
            Message::Scrolled(bottom) => self.follow_bottom = bottom,
            Message::Composing(active) => self.core.prompt_composing = active,
            Message::Clock => {}
            Message::Login => {
                if self.core.ready
                    && !self
                        .core
                        .pending
                        .values()
                        .any(|p| matches!(p, Pending::Login))
                {
                    self.core.error = None;
                    self.core.rpc(
                        "account/login/start",
                        json!({"type":"chatgpt"}),
                        Pending::Login,
                    );
                }
            }
            Message::CancelLogin => {
                if let Some(id) = &self.core.login_id {
                    self.core.rpc(
                        "account/login/cancel",
                        json!({"loginId":id}),
                        Pending::CancelLogin,
                    );
                } else {
                    self.core.auth_url = None;
                }
            }
            Message::Window(id, window::Event::CloseRequested) if id == self.window => {
                task = self.hide()
            }
            Message::Window(_, window::Event::Resized(size)) => {
                self.width = size.width;
                self.height = size.height;
                if size.width < 760. {
                    self.sidebar = false;
                }
            }
            Message::Window(_, window::Event::Focused) => self.core.hidden = false,
            Message::Window(_, _) => {}
            Message::Page(page) => {
                self.popup = None;
                self.page = page;
                if page == Page::Usage {
                    self.core.refresh_rate_limits();
                }
                if page == Page::Global && self.core.global.docs.is_none() {
                    self.core.global.load(&self.wake);
                }
                if page == Page::Settings {
                    if self.core.ready {
                        self.core.rpc(
                            "skills/list",
                            json!({"cwds":[self.core.settings.cwd],"forceReload":false}),
                            Pending::LocalSkills,
                        );
                    }
                    if self.core.global.docs.is_none() {
                        self.core.global.load(&self.wake);
                    }
                }
                if page == Page::Resources {
                    self.refresh_resources();
                }
            }
            Message::Sidebar => self.sidebar = !self.sidebar,
            Message::Connect => {
                self.popup = None;
                if self.core.binary == "codex" {
                    self.core.binary = crate::platform::codex_binary();
                }
                self.core.connect(&self.wake);
            }
            Message::Disconnect => {
                self.popup = None;
                self.core.disconnect();
            }
            Message::NewChat => {
                self.core.new_chat();
                self.reset_chat();
            }
            Message::SelectProject(id) => {
                self.core.switch_project(id);
                self.reset_chat();
            }
            Message::Resume(id) => {
                if let Some(recent) = self
                    .core
                    .settings
                    .recent
                    .iter()
                    .find(|r| r.id == id)
                    .cloned()
                {
                    self.core.resume(recent);
                    self.reset_chat();
                }
            }
            Message::Activate(index) => {
                self.core.activate_session(index);
                self.reset_chat();
            }
            Message::Compose(action) => {
                self.composer.perform(action);
                let prompt = self.composer.text();
                if prompt.len() <= 128 * 1024 {
                    self.core.prompt = prompt;
                }
            }
            Message::Send => {
                self.core.submit();
                self.follow_bottom = true;
            }
            Message::Stop => {
                if let (Some(thread), Some(turn)) =
                    (self.core.thread_id.clone(), self.core.turn_id.clone())
                {
                    self.core.rpc(
                        "turn/interrupt",
                        json!({"threadId":thread,"turnId":turn}),
                        Pending::Interrupt,
                    );
                }
            }
            Message::Model(model) => {
                self.popup = None;
                self.core.settings.model = model;
                self.core.settings.effort.clear();
                self.core.save();
            }
            Message::Effort(effort) => {
                self.popup = None;
                self.core.settings.effort = effort;
                if let Some(r) = self
                    .core
                    .settings
                    .recent
                    .iter_mut()
                    .find(|r| Some(&r.id) == self.core.thread_id.as_ref())
                {
                    r.effort = self.core.settings.effort.clone();
                }
                self.core.save();
            }
            Message::AutoAccept(enabled) => {
                self.core.auto_accept = enabled;
                self.core.accept_pending_automatically();
            }
            Message::Expand(id) => {
                if !self.expanded.remove(&id) {
                    self.expanded.insert(id);
                }
                self.follow_bottom = false;
            }
            Message::Raw(id) => {
                if self.raw.remove(&id).is_none()
                    && let Some(message) = self.core.transcript.messages.iter().find(|m| m.id == id)
                {
                    self.raw
                        .insert(id, text_editor::Content::with_text(&message.body));
                }
                self.follow_bottom = false;
            }
            Message::SelectText(id, action) => {
                if !action.is_edit()
                    && let Some(editor) = self.raw.get_mut(&id)
                {
                    editor.perform(action);
                }
            }
            Message::Copy(value) => task = iced::clipboard::write(value),
            Message::Link(url) => {
                if url.starts_with("https://") || url.starts_with("http://") {
                    task = Task::perform(
                        async move { open::that_detached(url).map_err(|e| e.to_string()) },
                        Message::LinkOpened,
                    );
                }
            }
            Message::LinkOpened(result) => {
                if let Err(error) = result {
                    self.core.error = Some(format!("브라우저를 열지 못했습니다: {error}"));
                }
            }
            Message::Older => {
                self.core.older_history();
                self.follow_bottom = false;
                self.visible = 160;
            }
            Message::MoreVisible => {
                self.visible = (self.visible + 40).min(160);
                self.follow_bottom = false;
            }
            Message::Latest => {
                if self.core.thread_id.is_some() && !self.core.busy && !self.core.loading {
                    self.core.history_cursor = None;
                    self.core.loading = self.core.rpc("thread/turns/list",json!({"threadId":self.core.thread_id,"limit":20,"itemsView":"full","sortDirection":"desc"}),Pending::History { older: false });
                }
                self.visible = 40;
                self.follow_bottom = true;
            }
            Message::ProjectNew => {
                self.core.project_editor = Some(Project {
                    id: 0,
                    name: String::new(),
                    cwd: self.core.settings.cwd.clone(),
                    instructions: String::new(),
                    efficiency: Default::default(),
                });
                self.instructions = text_editor::Content::new();
                self.core.project_error = None;
                self.page = Page::Project;
            }
            Message::ProjectEdit => {
                self.core.project_editor = self.core.settings.active().cloned();
                self.instructions = text_editor::Content::with_text(
                    self.core
                        .project_editor
                        .as_ref()
                        .map_or("", |p| &p.instructions),
                );
                self.core.project_error = None;
                self.page = Page::Project;
            }
            Message::ProjectField(field, value) => {
                if let Some(p) = &mut self.core.project_editor {
                    if field == 0 {
                        p.name = value;
                    } else {
                        p.cwd = value;
                    }
                }
            }
            Message::ProjectInstructions(action) => {
                self.instructions.perform(action);
                if let Some(p) = &mut self.core.project_editor {
                    p.instructions = self.instructions.text();
                }
            }
            Message::ProjectSave => match self.save_project() {
                Ok(()) => self.page = Page::Chat,
                Err(e) => self.core.project_error = Some(e),
            },
            Message::Language(english) => {
                self.core.settings.language = if english {
                    Language::English
                } else {
                    Language::Korean
                };
                self.core.save();
            }
            Message::Light(light) => {
                self.core.settings.theme = if light { Theme::Light } else { Theme::Dark };
                self.core.save();
            }
            Message::Notifications(enabled) => {
                self.core.settings.notifications = enabled;
                self.core.save();
            }
            Message::Concurrency(value) => {
                self.core.settings.max_concurrent_tasks = value.clamp(1, 8);
                self.core.save();
                self.core.tick_logic(&self.wake);
            }
            Message::GlobalTab(tab) => {
                self.core.global.tab = tab;
                self.global_source.clear();
            }
            Message::SkillFilter(filter) => self.core.global.filter = filter,
            Message::McpArguments(action) => {
                self.mcp_arguments.perform(action);
                self.core.global.mcp_args = self.mcp_arguments.text();
            }
            Message::GlobalEdit(action) => {
                if self.global_idle() {
                    self.global_editor.perform(action);
                    let value = self.global_editor.text();
                    if self.core.global.tab == 0 {
                        if let Some(d) = &mut self.core.global.docs {
                            d.agents.text = value;
                        }
                    } else {
                        self.core.global.edit_config(value);
                    }
                }
            }
            Message::GlobalField(key, value) => {
                if self.global_idle()
                    && let Some(d) = &mut self.core.global.docs
                    && d.config_valid
                {
                    if value.is_empty() {
                        d.parsed.remove(&key);
                    } else {
                        d.parsed[&key] = toml_edit::value(value);
                    }
                    d.config.text = d.parsed.to_string();
                }
            }
            Message::GlobalSave(config) => {
                if self.global_idle() {
                    self.core.global.save_document(&self.wake, config);
                }
            }
            Message::GlobalReload => {
                if self
                    .core
                    .global
                    .docs
                    .as_ref()
                    .is_none_or(|d| !d.agents.dirty() && !d.config.dirty())
                {
                    self.core.global.load(&self.wake);
                }
            }
            Message::Binary(value) => self.core.global.binary_draft = Some(value),
            Message::SaveBinary => {
                if self.global_idle()
                    && let Some(value) = &self.core.global.binary_draft
                    && !value.trim().is_empty()
                {
                    self.core.binary = value.trim().into();
                    self.core.save();
                }
            }
            Message::RefreshSkills => {
                self.core.rpc(
                    "skills/list",
                    json!({"cwds":[self.core.settings.cwd],"forceReload":true}),
                    Pending::Skills,
                );
            }
            Message::Skill(path, enabled) => {
                if self.global_idle()
                    && self
                        .core
                        .global
                        .docs
                        .as_ref()
                        .is_some_and(|d| !d.config.dirty())
                {
                    self.core.rpc(
                        "skills/config/write",
                        json!({"path":path,"enabled":enabled}),
                        Pending::SkillSaved,
                    );
                }
            }
            Message::LocalSkill(path, enabled) => {
                if let Some(p) = self
                    .core
                    .settings
                    .projects
                    .iter_mut()
                    .find(|p| Some(p.id) == self.core.settings.active_project)
                {
                    p.efficiency.disabled_skills.retain(|s| s != &path);
                    if !enabled {
                        p.efficiency.disabled_skills.push(path);
                    }
                    self.core.save();
                }
            }
            Message::McpField(field, value) => match field {
                0 => self.core.global.mcp_name = value,
                1 => self.core.global.mcp_command = value,
                2 => self.core.global.mcp_args = value,
                _ => self.core.global.mcp_url = value,
            },
            Message::McpHttp(value) => self.core.global.mcp_http = value,
            Message::McpApply => {
                if self.global_idle() {
                    let g = &mut self.core.global;
                    if let Some(d) = &mut g.docs
                        && d.config_valid
                    {
                        match crate::global::update_mcp(
                            &mut d.parsed,
                            &g.mcp_name,
                            &g.mcp_command,
                            &g.mcp_args,
                            &g.mcp_url,
                            g.mcp_http,
                        ) {
                            Ok(()) => d.config.text = d.parsed.to_string(),
                            Err(e) => g.message = e,
                        }
                    }
                }
            }
            Message::McpEdit(name) => {
                let g = &mut self.core.global;
                if let Some(item) = g
                    .docs
                    .as_ref()
                    .and_then(|d| d.parsed.get("mcp_servers"))
                    .and_then(|s| s.get(&name))
                {
                    g.mcp_name = name;
                    g.mcp_command = item
                        .get("command")
                        .and_then(toml_edit::Item::as_str)
                        .unwrap_or("")
                        .into();
                    g.mcp_url = item
                        .get("url")
                        .and_then(toml_edit::Item::as_str)
                        .unwrap_or("")
                        .into();
                    g.mcp_http = !g.mcp_url.is_empty();
                    g.mcp_args = item
                        .get("args")
                        .and_then(toml_edit::Item::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default();
                }
            }
            Message::McpToggle(name, enabled) => {
                if self.global_idle()
                    && let Some(d) = &mut self.core.global.docs
                    && d.config_valid
                {
                    d.parsed["mcp_servers"][&name]["enabled"] = toml_edit::value(enabled);
                    d.config.text = d.parsed.to_string();
                }
            }
            Message::LocalMcp(name, enabled) => {
                if let Some(p) = self
                    .core
                    .settings
                    .projects
                    .iter_mut()
                    .find(|p| Some(p.id) == self.core.settings.active_project)
                {
                    p.efficiency.disabled_mcp.retain(|s| s != &name);
                    if !enabled {
                        p.efficiency.disabled_mcp.push(name);
                    }
                    self.core.save();
                }
            }
            Message::EditMcpExclusions(action) => self.mcp_exclusions.perform(action),
            Message::SaveMcpExclusions => {
                let names: Vec<String> = self
                    .mcp_exclusions
                    .text()
                    .lines()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
                if names.iter().all(|name| {
                    name.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                }) {
                    if let Some(p) = self
                        .core
                        .settings
                        .projects
                        .iter_mut()
                        .find(|p| Some(p.id) == self.core.settings.active_project)
                    {
                        p.efficiency.disabled_mcp = names;
                        self.core.save();
                    }
                } else {
                    self.core.error =
                        Some("MCP names: letters, digits, underscore or hyphen only".into());
                }
            }
            Message::ProjectEffort(effort) => {
                if let Some(p) = self
                    .core
                    .settings
                    .projects
                    .iter_mut()
                    .find(|p| Some(p.id) == self.core.settings.active_project)
                {
                    p.efficiency.effort = effort;
                    self.core.save();
                }
            }
            Message::Rtk(enabled) => {
                if let Some(p) = self
                    .core
                    .settings
                    .projects
                    .iter_mut()
                    .find(|p| Some(p.id) == self.core.settings.active_project)
                {
                    p.efficiency.rtk = enabled;
                    self.core.save();
                }
            }
            Message::Answer(id, value) => {
                if let Some(a) = self.core.approvals.front_mut() {
                    a.answers.insert(id, value);
                }
            }
            Message::Approve(accept) => self.approve(accept),
            Message::AnswerSend => {
                if let Some(a) = self.core.approvals.front()
                    && answers_complete(a)
                {
                    let answers: serde_json::Map<String, Value> = a
                        .answers
                        .iter()
                        .map(|(k, v)| (k.clone(), json!({"answers":[v]})))
                        .collect();
                    self.core.respond(json!({"answers":answers}));
                }
            }
            Message::CopyUsage => {
                task = iced::clipboard::write(
                    serde_json::to_string_pretty(&self.usage_records()).unwrap_or_default(),
                )
            }
            Message::RefreshUsage => self.core.refresh_rate_limits(),
            Message::ChatOptions(id) => {
                self.chat_options = if self.chat_options.as_ref() == Some(&id) {
                    None
                } else {
                    Some(id)
                }
            }
            Message::MoveChat(id, project) => {
                self.core.move_chat(&id, project);
                self.chat_options = None;
            }
            Message::DeleteChat(id) => {
                self.core.delete_chat = self
                    .core
                    .settings
                    .recent
                    .iter()
                    .find(|r| r.id == id)
                    .cloned();
            }
            Message::ConfirmDelete => {
                if let Some(recent) = self.core.delete_chat.clone()
                    && !self.core.busy
                    && !self.core.loading
                {
                    self.core.loading = self.core.rpc(
                        "thread/delete",
                        json!({"threadId":recent.id}),
                        Pending::Delete(recent.id),
                    );
                }
            }
            Message::CancelDelete => self.core.delete_chat = None,
            Message::Compact => self.core.compact(),
            Message::CancelQueued(index) => {
                let job = if let Some(i) = index {
                    self.core
                        .sessions
                        .get_mut(i)
                        .and_then(|s| s.queued_job.take())
                } else {
                    self.core.queued_job.take()
                };
                if let Some(job) = job {
                    if let Some(i) = index {
                        if let Some(s) = self.core.sessions.get_mut(i) {
                            if s.prompt.is_empty() {
                                s.prompt = job.prompt;
                            } else {
                                s.prompt.push_str(&format!("\n\n{}", job.prompt));
                            }
                        }
                    } else if self.core.prompt.is_empty() {
                        self.core.prompt = job.prompt;
                    } else {
                        self.core.prompt.push_str(&format!("\n\n{}", job.prompt));
                    }
                }
            }
            Message::Hide => self.core.desktop.request(crate::desktop::Action::Hide),
            Message::Quit => task = self.quit(),
            Message::ConfirmQuit => task = self.exit(),
            Message::CancelQuit => self.core.show_exit_confirmation = false,
            Message::RefreshResources => self.refresh_resources(),
        }
        self.sync();
        if self.core.ready
            && self.core.pending.is_empty()
            && let Some(path) = self.benchmark_output.take()
        {
            let report = json!({
                "ready": true,
                "startup_ms": self.booted.elapsed().as_secs_f64() * 1000.,
                "backend_pid": self.core.backend.as_ref().map(|b| b.pid()),
                "models": self.core.models.len(),
            });
            if let Err(error) = std::fs::write(path, report.to_string()) {
                eprintln!("Benchmark readiness report failed: {error}");
                task = self.exit();
            }
        }
        if self.follow_bottom && self.page == Page::Chat && !self.core.hidden {
            task = Task::batch([task, widget::operation::snap_to_end(self.scroll.clone())]);
        }
        task
    }

    fn reset_chat(&mut self) {
        self.popup = None;
        self.page = Page::Chat;
        self.visible = 40;
        self.follow_bottom = true;
        self.chat_options = None;
        self.markdown.clear();
        self.expanded.clear();
        self.raw.clear();
        if self.width < 760.0 {
            self.sidebar = false;
        }
    }
    fn global_idle(&self) -> bool {
        self.smoke_output.is_none()
            && self.core.running_count() == 0
            && !self.core.loading
            && self.core.global.pending.is_none()
    }
    fn refresh_resources(&mut self) {
        let pids = self
            .core
            .backend
            .iter()
            .map(|b| b.pid())
            .chain(
                self.core
                    .sessions
                    .iter()
                    .filter_map(|s| s.backend.as_ref().map(|b| b.pid())),
            )
            .collect();
        let monitor = self
            .monitor
            .get_or_insert_with(|| crate::resources::Monitor::new(self.wake.clone()));
        monitor.request(pids);
    }
    fn hide(&mut self) -> Task<Message> {
        self.core.hidden = true;
        if self.core.desktop.has_tray() {
            window::set_mode(self.window, window::Mode::Hidden)
        } else {
            window::minimize(self.window, true)
        }
    }
    fn show(&mut self) -> Task<Message> {
        self.core.hidden = false;
        Task::batch([
            window::set_mode(self.window, window::Mode::Windowed),
            window::minimize(self.window, false),
            window::gain_focus(self.window),
        ])
    }
    fn quit(&mut self) -> Task<Message> {
        if self.core.running_count() > 0
            || self.core.loading
            || !self.core.approvals.is_empty()
            || self
                .core
                .sessions
                .iter()
                .any(|s| s.loading || !s.approvals.is_empty() || s.queued_job.is_some())
            || self.core.queued_job.is_some()
        {
            self.core.show_exit_confirmation = true;
            self.show()
        } else {
            self.exit()
        }
    }
    fn exit(&mut self) -> Task<Message> {
        if self.smoke_output.is_some() {
            return iced::exit();
        }
        self.core.shutdown();
        self.core.exiting = true;
        iced::exit()
    }
    fn save_project(&mut self) -> Result<(), String> {
        let mut p = self
            .core
            .project_editor
            .clone()
            .ok_or("프로젝트를 선택하세요.")?;
        if p.name.trim().is_empty() || p.name.len() > 120 || p.instructions.len() > 16 * 1024 {
            return Err("이름은 1~120바이트, 지침은 16 KiB 이하로 입력하세요.".into());
        }
        p.cwd = std::fs::canonicalize(&p.cwd)
            .map_err(|_| "존재하는 작업 폴더를 입력하세요.")?
            .to_string_lossy()
            .into_owned();
        if !std::path::Path::new(&p.cwd).is_dir() {
            return Err("작업 폴더를 선택하세요.".into());
        }
        if p.id == 0 {
            let id = self
                .core
                .settings
                .add_project(&p.name, &p.cwd, &p.instructions)?;
            if !self.core.busy && !self.core.loading {
                self.core.switch_project(id);
            }
        } else {
            if self.core.busy || self.core.loading {
                return Err("작업이 끝난 뒤 프로젝트를 편집하세요.".into());
            }
            if let Some(existing) = self
                .core
                .settings
                .projects
                .iter_mut()
                .find(|v| v.id == p.id)
            {
                *existing = p.clone();
            }
            if self.core.settings.active_project == Some(p.id) {
                self.core.disconnect();
                self.core.settings.cwd = p.cwd;
                self.core.transcript = Default::default();
                self.core.connect(&self.wake);
            }
        }
        self.core.save();
        Ok(())
    }
    fn approve(&mut self, accept: bool) {
        let Some(a) = self.core.approvals.front() else {
            return;
        };
        if a.method == "item/tool/requestUserInput" {
            return;
        }
        if a.method == "item/permissions/requestApproval" {
            if accept && !a.params["permissions"].is_object() {
                return;
            }
            self.core.respond(json!({"permissions":if accept { a.params["permissions"].clone() } else { json!({}) },"scope":"turn"}));
        } else {
            let decision = if accept { "accept" } else { "decline" };
            if a.params["availableDecisions"]
                .as_array()
                .is_none_or(|xs| xs.iter().any(|v| v == decision))
            {
                self.core.respond(json!({"decision":decision}));
            }
        }
    }
}

fn answers_complete(a: &crate::controller::Approval) -> bool {
    a.method == "item/tool/requestUserInput"
        && a.params["questions"].as_array().is_some_and(|qs| {
            !qs.is_empty()
                && qs.iter().all(|q| {
                    a.answers
                        .get(string(q, "id"))
                        .is_some_and(|s| !s.trim().is_empty())
                })
        })
}

impl App {
    fn small_button<'a>(&self, label: &'a str, message: Message) -> widget::Button<'a, Message> {
        button(text(label).size(13))
            .padding([8, 10])
            .style(button::text)
            .on_press(message)
    }
    fn heading<'a>(&self, title: &'a str) -> Element<'a, Message> {
        text(title).size(24).into()
    }
    fn icon_button<'a>(
        &self,
        kind: crate::icons::Kind,
        label: &'a str,
        message: Message,
    ) -> Element<'a, Message> {
        let colors = self.core.settings.theme.colors();
        widget::tooltip(
            button(crate::icons::view(kind, colors.muted, 18.))
                .padding(8)
                .style(button::text)
                .on_press(message),
            text(label).size(12),
            widget::tooltip::Position::Bottom,
        )
        .into()
    }
    fn nav_style(
        &self,
        selected: bool,
    ) -> impl Fn(&iced::Theme, button::Status) -> button::Style + use<> {
        let c = self.core.settings.theme.colors();
        move |_, status| button::Style {
            background: if selected
                || matches!(status, button::Status::Hovered | button::Status::Pressed)
            {
                Some(c.hover.into())
            } else {
                None
            },
            text_color: c.text,
            border: iced::Border {
                radius: 8.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    fn surface(&self, radius: f32) -> impl Fn(&iced::Theme) -> container::Style + use<> {
        let c = self.core.settings.theme.colors();
        move |_| container::Style {
            background: Some(c.surface.into()),
            text_color: Some(c.text),
            border: iced::Border {
                radius: radius.into(),
                width: if radius == 12. { 1. } else { 0. },
                color: c.border,
            },
            ..Default::default()
        }
    }
    fn view(&self, _: window::Id) -> Element<'_, Message> {
        if self.core.hidden {
            return space().into();
        }
        let content = match self.page {
            Page::Chat => self.chat_view(),
            Page::Settings => self.settings_view(),
            Page::Project => self.project_view(),
            Page::Global => self.global_view(),
            Page::Usage => self.usage_view(),
            Page::Resources => self.resources_view(),
        };
        let main: Element<'_, Message> = if self.page == Page::Chat {
            content
        } else {
            column![
                container(row![
                    self.icon_button(
                        crate::icons::Kind::Back,
                        self.label("대화로 돌아가기", "Back to chat"),
                        Message::Page(Page::Chat)
                    ),
                    space().width(Fill)
                ])
                .padding([8, 12]),
                container(content)
                    .padding(if self.width < 700. { 16 } else { 32 })
                    .width(Fill)
                    .height(Fill)
            ]
            .into()
        };
        let main = container(main).width(Fill).height(Fill);
        let layout: Element<'_, Message> = if self.sidebar && self.width >= 760. {
            row![self.sidebar_view(), main].height(Fill).into()
        } else if self.sidebar {
            let backdrop =
                widget::mouse_area(container(space()).width(Fill).height(Fill).style(|_| {
                    container::Style {
                        background: Some(iced::Color::from_rgba(0., 0., 0., 0.4).into()),
                        ..Default::default()
                    }
                }))
                .on_press(Message::Sidebar);
            widget::stack![main, widget::opaque(backdrop), self.sidebar_view()].into()
        } else {
            main.into()
        };
        let mut root = column![layout].height(Fill);
        if self.core.show_exit_confirmation {
            root = root.push(
                container(
                    column![
                        text(self.label(
                            "진행 중인 작업을 종료할까요?",
                            "Stop ongoing tasks and quit?"
                        )),
                        row![
                            button(self.label("종료", "Quit")).on_press(Message::ConfirmQuit),
                            self.small_button(self.label("취소", "Cancel"), Message::CancelQuit)
                        ]
                        .spacing(8)
                    ]
                    .spacing(10),
                )
                .padding(16)
                .style(self.surface(12.)),
            );
        }
        root.into()
    }
    fn sidebar_view(&self) -> Element<'_, Message> {
        use crate::icons::Kind;
        let c = self.core.settings.theme.colors();
        let top = row![
            text("Codex Mini").size(15).width(Fill),
            self.icon_button(
                Kind::Sidebar,
                self.label("사이드바 닫기", "Close sidebar"),
                Message::Sidebar
            )
        ]
        .align_y(iced::Center);
        let mut items = column![
            button(
                row![
                    crate::icons::view(Kind::NewChat, c.text, 18.),
                    text(self.label("새 대화", "New chat")).size(14)
                ]
                .spacing(12)
                .align_y(iced::Center)
            )
            .padding([10, 12])
            .width(Fill)
            .style(self.nav_style(false))
            .on_press(Message::NewChat),
            space().height(22),
            row![
                text(self.label("프로젝트", "Projects"))
                    .size(12)
                    .color(c.muted)
                    .width(Fill),
                self.small_button("+", Message::ProjectNew)
            ]
            .align_y(iced::Center)
        ]
        .spacing(3);
        for p in &self.core.settings.projects {
            let active = Some(p.id) == self.core.settings.active_project;
            let running = self
                .core
                .sessions
                .iter()
                .any(|s| s.project_id == Some(p.id) && s.busy)
                || active && self.core.busy;
            items = items.push(
                button(
                    row![
                        text(&p.name).size(14).width(Fill),
                        text(if running { "·" } else { "" }).size(18)
                    ]
                    .align_y(iced::Center),
                )
                .padding([9, 12])
                .width(Fill)
                .style(self.nav_style(active))
                .on_press(Message::SelectProject(p.id)),
            );
        }
        items = items.push(space().height(22)).push(
            container(text(self.label("대화", "Chats")).size(12).color(c.muted)).padding([0, 12]),
        );
        for r in self
            .core
            .settings
            .recent
            .iter()
            .filter(|r| r.project_id == self.core.settings.active_project)
        {
            let needs_attention = self
                .core
                .sessions
                .iter()
                .any(|s| s.thread_id.as_deref() == Some(&r.id) && !s.approvals.is_empty());
            let title = r.title.lines().next().unwrap_or(&r.title);
            let title = format!(
                "{}{}",
                if needs_attention { "! " } else { "" },
                title.chars().take(25).collect::<String>()
            );
            items = items.push(
                row![
                    button(text(title).size(13))
                        .padding([10, 12])
                        .width(Fill)
                        .style(self.nav_style(self.core.thread_id.as_deref() == Some(&r.id)))
                        .on_press(Message::Resume(r.id.clone())),
                    self.icon_button(
                        Kind::More,
                        self.label("대화 메뉴", "Chat menu"),
                        Message::ChatOptions(r.id.clone())
                    )
                ]
                .align_y(iced::Center),
            );
            if self.chat_options.as_ref() == Some(&r.id) {
                let mut actions = column![
                    self.small_button(
                        self.label("ID 복사", "Copy ID"),
                        Message::Copy(r.id.clone())
                    ),
                    self.small_button(
                        self.label("삭제…", "Delete…"),
                        Message::DeleteChat(r.id.clone())
                    )
                ]
                .spacing(2);
                for p in self
                    .core
                    .settings
                    .projects
                    .iter()
                    .filter(|p| Some(p.id) != r.project_id)
                {
                    actions = actions.push(
                        button(
                            text(format!("{} → {}", self.label("이동", "Move"), p.name)).size(12),
                        )
                        .padding(8)
                        .style(button::text)
                        .on_press(Message::MoveChat(r.id.clone(), p.id)),
                    );
                }
                items = items.push(container(actions).padding([2, 12]).style(self.surface(10.)));
            }
        }
        if let Some(job) = &self.core.queued_job {
            items = items.push(self.queue_row(&job.prompt, None));
        }
        for (i, s) in self.core.sessions.iter().enumerate() {
            if let Some(job) = &s.queued_job {
                items = items.push(self.queue_row(&job.prompt, Some(i)));
            }
            if s.busy || s.loading || !s.approvals.is_empty() {
                let project = self
                    .core
                    .settings
                    .projects
                    .iter()
                    .find(|p| Some(p.id) == s.project_id)
                    .map_or("Codex", |p| p.name.as_str());
                let state = if s.approvals.is_empty() {
                    self.label("실행 중", "Running")
                } else {
                    self.label("확인 필요", "Needs attention")
                };
                items = items.push(
                    button(text(format!("{project} · {state}")).size(12))
                        .padding(10)
                        .style(button::text)
                        .on_press(Message::Activate(i)),
                );
            }
        }
        let foot = row![
            crate::icons::view(Kind::Settings, c.muted, 18.),
            text(self.label("설정", "Settings")).size(14)
        ]
        .spacing(12)
        .align_y(iced::Center);
        container(
            column![
                container(top).padding([4, 8]),
                space().height(8),
                scrollable(items).height(Fill),
                button(foot)
                    .width(Fill)
                    .padding([14, 12])
                    .style(self.nav_style(self.page == Page::Settings))
                    .on_press(Message::Page(Page::Settings))
            ]
            .spacing(4),
        )
        .padding(12)
        .width(if self.width < 760. { 210 } else { 244 })
        .height(Fill)
        .style(move |_| container::Style {
            background: Some(c.sidebar.into()),
            ..Default::default()
        })
        .into()
    }
    fn queue_row<'a>(&self, prompt: &'a str, index: Option<usize>) -> Element<'a, Message> {
        row![
            text(format!(
                "{} · {}",
                self.label("대기", "Queued"),
                prompt.chars().take(18).collect::<String>()
            ))
            .size(12)
            .width(Fill),
            self.small_button("×", Message::CancelQueued(index))
        ]
        .padding(8)
        .align_y(iced::Center)
        .into()
    }
    fn chat_view(&self) -> Element<'_, Message> {
        use crate::icons::Kind;
        let c = &self.core;
        let colors = c.settings.theme.colors();
        let model = if c.settings.model.is_empty() {
            "Codex"
        } else {
            c.models
                .iter()
                .find(|m| string(m, "model") == c.settings.model)
                .and_then(|m| m["displayName"].as_str())
                .unwrap_or(&c.settings.model)
        };
        let mut header = row![].spacing(6).align_y(iced::Center);
        if !self.sidebar {
            header = header
                .push(self.icon_button(
                    Kind::Sidebar,
                    self.label("사이드바 열기", "Open sidebar"),
                    Message::Sidebar,
                ))
                .push(self.icon_button(
                    Kind::NewChat,
                    self.label("새 대화", "New chat"),
                    Message::NewChat,
                ));
        }
        header = header
            .push(
                button(
                    row![
                        text(model).size(17),
                        crate::icons::view(Kind::Down, colors.muted, 12.)
                    ]
                    .spacing(6)
                    .align_y(iced::Center),
                )
                .padding([9, 10])
                .style(button::text)
                .on_press_maybe((c.ready && !c.needs_auth).then_some(Message::Menu(Popup::Model))),
            )
            .push(space().width(Fill));
        if !c.ready && !c.loading && !c.transcript.messages.is_empty() {
            header =
                header.push(self.small_button(self.label("연결", "Connect"), Message::Connect));
        }
        if c.ready && !c.needs_auth {
            header = header.push(
                self.small_button(self.label("사용량", "Usage"), Message::Menu(Popup::Usage)),
            );
        }
        header = header.push(self.icon_button(
            Kind::More,
            self.label("대화 메뉴", "Chat menu"),
            Message::Menu(Popup::Chat),
        ));
        let mut body = column![container(header).padding([6, 12])].height(Fill);
        if matches!(self.popup, Some(Popup::Model | Popup::Chat | Popup::Usage)) {
            body = body.push(container(self.popup_view()).padding([0, 24]));
        }
        let gutter = if self.width < 760. { 18 } else { 40 };
        let mut messages = column![].spacing(28);
        if c.history_cursor.is_some() {
            messages = messages.push(
                self.small_button(self.label("이전 대화", "Earlier messages"), Message::Older),
            );
        }
        if c.transcript.messages.len() > self.visible {
            messages = messages
                .push(self.small_button(self.label("더 보기", "Show more"), Message::MoreVisible));
        }
        if c.transcript.trimmed {
            messages = messages.push(
                text(self.label("일부 내용 생략됨", "Some content omitted"))
                    .size(12)
                    .color(colors.muted),
            );
        }
        for m in c
            .transcript
            .messages
            .iter()
            .skip(c.transcript.messages.len().saturating_sub(self.visible))
        {
            messages = messages.push(self.message_view(m));
        }
        if c.transcript.messages.is_empty() && c.approvals.is_empty() && (!c.ready || c.needs_auth)
        {
            body = body.push(
                container(self.setup_view())
                    .center_x(Fill)
                    .center_y(Fill)
                    .padding(gutter),
            );
        } else if c.transcript.messages.is_empty() && c.approvals.is_empty() {
            body = body.push(space().height(Fill)).push(
                container(
                    text(self.label("무엇을 작업할까요?", "What are we working on?")).size(26),
                )
                .center_x(Fill),
            );
            body = body
                .push(
                    container(container(self.composer_view()).max_width(736).width(Fill))
                        .center_x(Fill)
                        .padding([24, gutter]),
                )
                .push(space().height(Fill));
        } else {
            let history = scrollable(
                container(container(messages).width(Fill).max_width(736))
                    .center_x(Fill)
                    .padding([24, gutter]),
            )
            .id(self.scroll.clone())
            .on_scroll(|v| Message::Scrolled(v.relative_offset().y >= 0.98))
            .height(Fill);
            body = body.push(history);
            let mut bottom = column![].spacing(12);
            if c.delete_chat.is_some() {
                bottom = bottom.push(
                    container(
                        column![
                            text(self.label(
                                "대화와 하위 대화를 삭제할까요?",
                                "Delete this chat and its child chats?"
                            )),
                            row![
                                button(self.label("삭제", "Delete"))
                                    .on_press(Message::ConfirmDelete),
                                self.small_button(
                                    self.label("취소", "Cancel"),
                                    Message::CancelDelete
                                )
                            ]
                            .spacing(8)
                        ]
                        .spacing(10),
                    )
                    .padding(14)
                    .style(self.surface(12.)),
                );
            }
            if !c.approvals.is_empty() {
                bottom = bottom.push(self.approval_view());
            }
            bottom = bottom.push(self.composer_view());
            body = body.push(
                container(container(bottom).max_width(736).width(Fill))
                    .center_x(Fill)
                    .padding([12, gutter]),
            );
        }
        body.into()
    }
    fn popup_view(&self) -> Element<'_, Message> {
        let c = &self.core;
        let mut content = column![].spacing(10);
        match self.popup {
            Some(Popup::Usage) => {
                let summary = container(scrollable(container(self.usage_summary()).padding(
                    iced::Padding {
                        right: 14.,
                        ..Default::default()
                    },
                )))
                .max_height((self.height - 310.).clamp(120., 340.));
                return container(
                    column![
                        summary,
                        self.small_button(
                            self.label("기록 보기", "View history"),
                            Message::Page(Page::Usage)
                        )
                    ]
                    .spacing(8),
                )
                .padding(14)
                .style(self.surface(12.))
                .into();
            }
            Some(Popup::Model) => {
                let mut models = vec![String::new()];
                models.extend(c.models.iter().map(|m| string(m, "model").into()));
                if c.thread_id.is_none() && !c.busy && !c.loading {
                    content = content.push(choices(
                        models,
                        &c.settings.model,
                        self.label("기본 모델", "Default model"),
                        Message::Model,
                    ));
                } else {
                    content = content.push(text(&c.settings.model).size(14));
                }
                content = content.push(self.effort_selector());
            }
            Some(Popup::Composer) => {
                content = content.push(self.effort_selector()).push(
                    checkbox(c.auto_accept)
                        .label(self.label("자동수락", "Auto-accept"))
                        .on_toggle(Message::AutoAccept)
                        .text_size(14),
                );
            }
            _ => {
                content = content
                    .push(text(c.settings.active().map_or("", |p| p.name.as_str())).size(14))
                    .push(
                        text(&c.settings.cwd)
                            .size(12)
                            .color(c.settings.theme.colors().muted),
                    );
                // Build a normal row before wrapping so each action keeps its own hit area.
                let buttons = row![
                    self.small_button(
                        self.label("프로젝트 편집", "Edit project"),
                        Message::ProjectEdit
                    ),
                    self.small_button(self.label("컨텍스트 압축", "Compact"), Message::Compact),
                    self.small_button(self.label("사용량", "Usage"), Message::Page(Page::Usage)),
                    self.small_button(
                        self.label("리소스", "Resources"),
                        Message::Page(Page::Resources)
                    )
                ]
                .spacing(4);
                content = content.push(buttons.wrap());
                content = content.push(
                    row![
                        self.small_button(
                            self.label(
                                if c.ready { "연결 종료" } else { "연결" },
                                if c.ready { "Disconnect" } else { "Connect" }
                            ),
                            if c.ready {
                                Message::Disconnect
                            } else {
                                Message::Connect
                            }
                        ),
                        self.small_button(self.label("숨기기", "Hide"), Message::Hide),
                        self.small_button(self.label("종료", "Quit"), Message::Quit)
                    ]
                    .spacing(4),
                );
            }
        }
        container(content)
            .padding(14)
            .style(self.surface(12.))
            .into()
    }
    fn effort_selector(&self) -> Element<'_, Message> {
        let c = &self.core;
        let model = c
            .models
            .iter()
            .find(|m| string(m, "model") == c.settings.model)
            .or_else(|| c.models.iter().find(|m| m["isDefault"] == true));
        let mut efforts = vec![String::new()];
        efforts.extend(
            model
                .and_then(|m| m["supportedReasoningEfforts"].as_array())
                .into_iter()
                .flatten()
                .map(|e| string(e, "reasoningEffort").into()),
        );
        let selector: Element<'_, Message> = row![
            text(self.label("추론", "Reasoning")).size(14),
            choices(
                efforts,
                &c.settings.effort,
                self.label("기본값", "Default"),
                Message::Effort
            )
        ]
        .spacing(12)
        .align_y(iced::Center)
        .into();
        if let Some(project) = c.settings.active()
            && let Some(model) = crate::recommendation::model(&c.models, &project.instructions)
        {
            widget::tooltip(
                selector,
                text(format!(
                    "{}: {} · {}",
                    self.label("권장", "Suggested"),
                    string(model, "model"),
                    crate::recommendation::supported_effort(model, &project.instructions)
                ))
                .size(12),
                widget::tooltip::Position::Bottom,
            )
            .into()
        } else {
            selector
        }
    }
    fn message_view<'a>(&'a self, m: &'a state::Message) -> Element<'a, Message> {
        use crate::icons::Kind;
        let colors = self.core.settings.theme.colors();
        let user = m.title == "나";
        let mut content = column![].spacing(12);
        if m.tool {
            let title = m.body.lines().next().unwrap_or(&m.title);
            let failed = m.title.contains("failed") || m.title.contains("error");
            let expanded = self.expanded.contains(&m.id);
            content = content.push(
                button(
                    row![
                        crate::icons::view(Kind::Terminal, colors.muted, 16.),
                        text(title.chars().take(76).collect::<String>())
                            .size(13)
                            .color(colors.muted),
                        space().width(Fill),
                        text(if failed {
                            self.label("실패", "Failed")
                        } else {
                            ""
                        })
                        .size(12),
                        crate::icons::view(Kind::Chevron, colors.muted, 12.)
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                )
                .width(Fill)
                .padding([8, 0])
                .style(button::text)
                .on_press(Message::Expand(m.id.clone())),
            );
            if expanded {
                content = content.push(
                    container(text(&m.body).font(iced::Font::MONOSPACE).size(13))
                        .width(Fill)
                        .padding(14)
                        .style(self.surface(10.)),
                );
                content = content.push(row![
                    self.icon_button(
                        Kind::Copy,
                        self.label("복사", "Copy"),
                        Message::Copy(m.body.clone())
                    ),
                    self.icon_button(
                        Kind::Source,
                        self.label("원문 선택", "Select source"),
                        Message::Raw(m.id.clone())
                    )
                ]);
            } else if failed {
                let tail = m
                    .body
                    .lines()
                    .rev()
                    .take(6)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n");
                content = content.push(text(tail).font(iced::Font::MONOSPACE).size(13));
            }
        } else if let Some(editor) = self.raw.get(&m.id) {
            let id = m.id.clone();
            content = content.push(
                text_editor(editor)
                    .on_action(move |a| Message::SelectText(id.clone(), a))
                    .size(14)
                    .height(220),
            );
        } else if user {
            content = content.push(text(&m.body).size(16).line_height(1.5));
        } else {
            let key = format!(
                "{}/{}",
                self.core.thread_id.as_deref().unwrap_or("new"),
                m.id
            );
            if let Some(cache) = self.markdown.get(&key) {
                content = content.push(
                    markdown::view(cache.parsed.items(), self.core.settings.theme.iced())
                        .map(Message::Link),
                );
            } else {
                content = content.push(text(&m.body).size(16).line_height(1.5));
            }
        }
        if !m.tool {
            let actions = row![
                self.icon_button(
                    Kind::Copy,
                    self.label("복사", "Copy"),
                    Message::Copy(m.body.clone())
                ),
                self.icon_button(
                    Kind::Source,
                    self.label("원문 선택", "Select source"),
                    Message::Raw(m.id.clone())
                )
            ]
            .spacing(2);
            if user {
                return container(
                    column![
                        container(content)
                            .max_width(540)
                            .padding([12, 18])
                            .style(self.surface(20.)),
                        actions
                    ]
                    .spacing(4)
                    .align_x(iced::Right),
                )
                .width(Fill)
                .align_x(iced::Right)
                .into();
            }
            content = content.push(actions);
        }
        if m.tool && self.raw.contains_key(&m.id) {
            let id = m.id.clone();
            content = content.push(
                text_editor(&self.raw[&m.id])
                    .on_action(move |a| Message::SelectText(id.clone(), a))
                    .height(200)
                    .size(13),
            );
        }
        content.width(Fill).into()
    }
    fn composer_view(&self) -> Element<'_, Message> {
        use crate::icons::Kind;
        let c = &self.core;
        let colors = c.settings.theme.colors();
        let editor = text_editor(&self.composer)
            .id(self.composer_id.clone())
            .placeholder(self.label("메시지 입력", "Message Codex"))
            .on_action(Message::Compose)
            .key_binding(move |event| {
                if c.prompt_composing
                    && event.key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
                {
                    None
                } else {
                    composer_binding(event)
                }
            })
            .size(16)
            .line_height(1.5)
            .padding(0)
            .height(Length::Fixed(if c.approvals.is_empty() {
                (self.composer.line_count().min(5) as f32 * 24.).clamp(42., 120.)
            } else {
                34.
            }))
            .style(move |_, _| text_editor::Style {
                background: iced::Color::TRANSPARENT.into(),
                border: iced::Border::default(),
                placeholder: colors.muted,
                value: colors.text,
                selection: colors.hover,
            });
        let mut controls = row![self.icon_button(
            Kind::Sliders,
            self.label("대화 옵션", "Chat options"),
            Message::Menu(Popup::Composer)
        )]
        .spacing(8)
        .align_y(iced::Center);
        if c.auto_accept {
            controls = controls.push(
                text(self.label("자동수락 켜짐", "Auto-accept on"))
                    .size(12)
                    .color(colors.muted),
            );
        }
        controls = controls.push(space().width(Fill));
        if !self.follow_bottom {
            controls = controls.push(self.icon_button(
                Kind::Down,
                self.label("최신 메시지", "Latest messages"),
                Message::Latest,
            ));
        }
        if c.busy {
            controls = controls.push(self.icon_button(
                Kind::Stop,
                self.label("중단", "Stop"),
                Message::Stop,
            ));
        }
        let enabled = c.can_send() && !c.prompt.trim().is_empty() && c.queued_job.is_none();
        let send = button(crate::icons::view(
            Kind::Send,
            if enabled {
                colors.on_accent
            } else {
                colors.muted
            },
            18.,
        ))
        .padding(9)
        .on_press_maybe(enabled.then_some(Message::Send))
        .style(move |_, status| button::Style {
            background: Some(
                if !enabled || status == button::Status::Disabled {
                    colors.hover
                } else {
                    colors.accent
                }
                .into(),
            ),
            border: iced::Border {
                radius: 24.into(),
                ..Default::default()
            },
            ..Default::default()
        });
        controls = controls.push(widget::tooltip(
            send,
            text(self.label("전송", "Send")).size(12),
            widget::tooltip::Position::Top,
        ));
        let mut area = column![].spacing(10);
        if self.popup == Some(Popup::Composer) {
            area = area.push(self.popup_view());
        }
        if let Some(error) = &c.error {
            area = area.push(text(c.settings.language.message(error).into_owned()).size(13));
        }
        if c.needs_auth {
            area = area.push(self.small_button(self.label("로그인", "Sign in"), Message::Login));
        }
        if let Some(url) = &c.auth_url {
            area = area.push(self.small_button(
                self.label("브라우저에서 로그인", "Sign in with browser"),
                Message::Link(url.clone()),
            ));
        }
        if c.busy || c.loading || c.queued_job.is_some() {
            let status = if c.queued_job.is_some() {
                self.label("실행 대기", "Queued")
            } else if c.loading {
                self.label("연결 중", "Connecting")
            } else {
                self.label("작업 중", "Working")
            };
            let elapsed = if c.busy {
                c.started
                    .map(|t| format!(" · {}s", t.elapsed().as_secs()))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            area = area.push(
                text(format!("{status}{elapsed}"))
                    .size(12)
                    .color(colors.muted),
            );
        }
        area.push(
            container(column![editor, controls].spacing(8))
                .padding([14, 16])
                .width(Fill)
                .style(self.surface(24.)),
        )
        .into()
    }

    fn setup_view(&self) -> Element<'_, Message> {
        let c = &self.core;
        let colors = c.settings.theme.colors();
        let mut content = column![].spacing(18).max_width(440).align_x(iced::Center);
        if c.loading {
            return content
                .push(text(self.label("Codex 연결 중…", "Connecting to Codex…")).size(24))
                .into();
        }
        if c.needs_auth {
            content =
                content.push(text(self.label("ChatGPT로 로그인", "Sign in with ChatGPT")).size(26));
            if let Some(url) = &c.auth_url {
                content = content
                    .push(
                        text(self.label(
                            "브라우저에서 로그인을 완료하세요.",
                            "Finish signing in in your browser.",
                        ))
                        .color(colors.muted),
                    )
                    .push(
                        row![
                            self.small_button(
                                self.label("브라우저 열기", "Open browser"),
                                Message::Link(url.clone())
                            ),
                            self.small_button(
                                self.label("링크 복사", "Copy link"),
                                Message::Copy(url.clone())
                            ),
                            self.small_button(self.label("취소", "Cancel"), Message::CancelLogin)
                        ]
                        .spacing(8),
                    );
            } else {
                let pending = c.pending.values().any(|p| matches!(p, Pending::Login));
                content = content.push(
                    button(text(self.label("로그인", "Sign in")).size(14))
                        .padding([11, 24])
                        .on_press_maybe((!pending).then_some(Message::Login)),
                );
            }
        } else {
            content = content
                .push(text(self.label("Codex 연결", "Connect to Codex")).size(26))
                .push(
                    button(text(self.label("연결", "Connect")).size(14))
                        .padding([11, 24])
                        .on_press(Message::Connect),
                )
                .push(
                    row![
                        self.small_button(
                            self.label("CLI 설치 안내", "Install Codex CLI"),
                            Message::Link("https://learn.chatgpt.com/docs/cli".into())
                        ),
                        self.small_button(
                            self.label("CLI 경로 설정", "Set CLI path"),
                            Message::Page(Page::Global)
                        )
                    ]
                    .spacing(8),
                );
        }
        if let Some(error) = &c.error {
            content = content.push(
                text(
                    c.settings
                        .language
                        .message(&state::clipped(error, 500))
                        .into_owned(),
                )
                .size(13)
                .color(colors.muted),
            );
        }
        content.into()
    }

    fn approval_view(&self) -> Element<'_, Message> {
        let a = self.core.approvals.front().expect("approval");
        let mut details = column![].spacing(8);
        let actions: Element<'_, Message>;
        if a.method == "item/tool/requestUserInput" {
            for q in a.params["questions"].as_array().into_iter().flatten() {
                let id = string(q, "id").to_owned();
                details = details.push(text(string(q, "question")).size(13));
                let mut options = row![].spacing(5);
                for option in q["options"].as_array().into_iter().flatten() {
                    let label = string(option, "label");
                    options = options
                        .push(self.small_button(label, Message::Answer(id.clone(), label.into())));
                }
                details = details.push(options.wrap()).push(
                    text_input(
                        self.label("답변 직접 입력", "Type an answer"),
                        a.answers.get(&id).map_or("", String::as_str),
                    )
                    .on_input(move |v| Message::Answer(id.clone(), v)),
                );
            }
            actions = button(self.label("답변 보내기", "Send answers"))
                .on_press_maybe(answers_complete(a).then_some(Message::AnswerSend))
                .into();
        } else {
            if !a.preview_complete {
                details = details.push(
                    text(self.label(
                        "미리보기가 비어 있거나 생략되었습니다. 직접 확인 후 결정하세요.",
                        "The preview is empty or truncated. Review before deciding.",
                    ))
                    .size(12)
                    .style(text::danger),
                );
            }
            details = details.push(text(&a.preview).font(iced::Font::MONOSPACE).size(12));
            let permissions = a.method == "item/permissions/requestApproval";
            let accept = if permissions {
                a.params["permissions"].is_object()
            } else {
                a.params["availableDecisions"]
                    .as_array()
                    .is_none_or(|ds| ds.iter().any(|v| v == "accept"))
            };
            let decline = permissions
                || a.params["availableDecisions"]
                    .as_array()
                    .is_none_or(|ds| ds.iter().any(|v| v == "decline"));
            actions = row![
                button(self.label("이번 작업 허용", "Allow once"))
                    .on_press_maybe(accept.then_some(Message::Approve(true))),
                button(self.label("거절", "Decline"))
                    .on_press_maybe(decline.then_some(Message::Approve(false)))
                    .style(button::secondary)
            ]
            .spacing(8)
            .into();
        }
        container(
            column![
                text(self.label("확인·답변 필요", "Your attention is needed")).size(15),
                scrollable(details)
                    .width(Fill)
                    .height(Length::Fixed((self.height - 380.0).clamp(45.0, 160.0))),
                actions
            ]
            .spacing(8)
            .width(Fill),
        )
        .width(Fill)
        .padding(12)
        .style(container::bordered_box)
        .into()
    }
}

fn composer_binding(event: text_editor::KeyPress) -> Option<text_editor::Binding<Message>> {
    if matches!(event.status, text_editor::Status::Focused { .. })
        && event.key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
        && !event.modifiers.shift()
    {
        Some(text_editor::Binding::Custom(Message::Send))
    } else {
        text_editor::Binding::from_key_press(event)
    }
}

impl App {
    fn settings_row<'a>(
        &self,
        label: impl Into<String>,
        control: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        row![
            container(text(label.into()).size(14))
                .width(136)
                .height(34)
                .align_y(iced::Center),
            container(row![control.into(), space().width(0).height(34)].align_y(iced::Center))
                .width(Fill)
                .align_y(iced::Center)
        ]
        .spacing(20)
        .align_y(iced::Top)
        .into()
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let c = &self.core;
        let language = if c.settings.language == Language::English {
            "English"
        } else {
            "한국어"
        };
        let light = self.label("밝게", "Light");
        let dark = self.label("어둡게", "Dark");
        let mut content = column![
            self.heading(self.label("설정", "Settings")),
            text(self.label("일반", "General")).size(16),
            self.settings_row(
                self.label("언어", "Language"),
                container(
                    pick_list(vec!["한국어", "English"], Some(language), |v| {
                        Message::Language(v == "English")
                    })
                    .text_size(14)
                    .width(Fill)
                )
                .width(240)
            ),
            self.settings_row(
                self.label("테마", "Theme"),
                container(
                    pick_list(
                        vec![dark, light],
                        Some(if c.settings.theme == Theme::Light {
                            light
                        } else {
                            dark
                        }),
                        move |v| Message::Light(v == light)
                    )
                    .text_size(14)
                    .width(Fill)
                )
                .width(240)
            ),
            self.settings_row(
                self.label("작업 알림", "Notifications"),
                checkbox(c.settings.notifications)
                    .label(self.label(
                        "완료 · 실패 · 승인 요청",
                        "Completion · failure · approvals"
                    ))
                    .text_size(14)
                    .on_toggle(Message::Notifications)
            ),
            self.settings_row(
                self.label("동시 실행", "Concurrent tasks"),
                container(
                    pick_list(
                        vec![1usize, 2, 3, 4, 8],
                        Some(c.settings.max_concurrent_tasks),
                        Message::Concurrency
                    )
                    .text_size(14)
                    .width(Fill)
                )
                .width(240)
            ),
            widget::rule::horizontal(1)
        ]
        .spacing(14);
        if let Some(p) = c.settings.active() {
            let model = c
                .models
                .iter()
                .find(|m| string(m, "model") == c.settings.model)
                .or_else(|| c.models.iter().find(|m| m["isDefault"] == true));
            let mut efforts = vec![String::new()];
            efforts.extend(
                model
                    .and_then(|m| m["supportedReasoningEfforts"].as_array())
                    .into_iter()
                    .flatten()
                    .map(|e| string(e, "reasoningEffort").into()),
            );
            content = content
                .push(
                    text(format!(
                        "{} · {}",
                        self.label("프로젝트", "Project"),
                        p.name
                    ))
                    .size(16),
                )
                .push(
                    self.settings_row(
                        self.label("기본 추론", "Reasoning effort"),
                        container(choices(
                            efforts,
                            &p.efficiency.effort,
                            self.label("Codex 기본값", "Codex default"),
                            Message::ProjectEffort,
                        ))
                        .width(240),
                    ),
                )
                .push(
                    self.settings_row(
                        self.label("Cargo 로그", "Cargo output"),
                        checkbox(p.efficiency.rtk)
                            .label(self.label("RTK 필터", "RTK filter"))
                            .text_size(14)
                            .on_toggle_maybe(
                                (c.rtk_path.is_some() || p.efficiency.rtk).then_some(Message::Rtk),
                            ),
                    ),
                );
            for skill in &c.local_skills {
                let path = string(skill, "path").to_owned();
                let enabled = skill["enabled"].as_bool().unwrap_or(true)
                    && !p.efficiency.disabled_skills.contains(&path);
                content = content.push(
                    self.settings_row(
                        self.label("스킬", "Skill"),
                        checkbox(enabled)
                            .label(string(skill, "name"))
                            .text_size(14)
                            .on_toggle(move |v| Message::LocalSkill(path.clone(), v)),
                    ),
                );
            }
            if let Some(servers) = c
                .global
                .docs
                .as_ref()
                .and_then(|d| d.parsed.get("mcp_servers"))
                .and_then(toml_edit::Item::as_table_like)
            {
                for (name, _) in servers.iter() {
                    let name = name.to_owned();
                    content = content.push(
                        self.settings_row(
                            "MCP",
                            checkbox(!p.efficiency.disabled_mcp.contains(&name))
                                .label(name.clone())
                                .text_size(14)
                                .on_toggle(move |v| Message::LocalMcp(name.clone(), v)),
                        ),
                    );
                }
            }
            content =
                content.push(
                    self.settings_row(
                        self.label("제외할 MCP", "Excluded MCP"),
                        column![
                            text_editor(&self.mcp_exclusions)
                                .on_action(Message::EditMcpExclusions)
                                .height(90)
                                .size(14)
                                .placeholder(self.label(
                                    "서버 이름을 한 줄에 하나씩",
                                    "One server name per line"
                                )),
                            row![
                                space().width(Fill),
                                button(text(self.label("저장", "Save")).size(13))
                                    .padding([8, 18])
                                    .on_press(Message::SaveMcpExclusions)
                            ]
                        ]
                        .spacing(8),
                    ),
                );
        }
        content = content.push(widget::rule::horizontal(1)).push(
            self.settings_row(
                self.label("관리", "Manage"),
                row![
                    self.small_button(
                        self.label("전역 Codex 설정", "Global Codex settings"),
                        Message::Page(Page::Global)
                    ),
                    self.small_button(self.label("사용량", "Usage"), Message::Page(Page::Usage)),
                    self.small_button(
                        self.label("리소스", "Resources"),
                        Message::Page(Page::Resources)
                    )
                ]
                .spacing(4)
                .wrap(),
            ),
        );
        scrollable(
            container(content)
                .max_width(740)
                .width(Fill)
                .padding(iced::Padding {
                    right: 14.,
                    ..Default::default()
                }),
        )
        .into()
    }

    fn project_view(&self) -> Element<'_, Message> {
        let Some(p) = &self.core.project_editor else {
            return text(self.label("프로젝트를 선택하세요.", "Select a project.")).into();
        };
        let mut content = column![
            self.heading(self.label(
                if p.id == 0 {
                    "프로젝트 추가"
                } else {
                    "프로젝트 편집"
                },
                if p.id == 0 {
                    "New project"
                } else {
                    "Edit project"
                }
            )),
            text(self.label("이름", "Name")).size(12),
            text_input(self.label("프로젝트 이름", "Project name"), &p.name)
                .on_input(|v| Message::ProjectField(0, v)),
            text(self.label("작업 폴더", "Working directory")).size(12),
            text_input("/path/to/project", &p.cwd).on_input(|v| Message::ProjectField(1, v)),
            text(self.label("프로젝트 지침 · 선택", "Project instructions · optional")).size(12),
            text_editor(&self.instructions)
                .on_action(Message::ProjectInstructions)
                .height(220)
                .size(13),
            row![
                button(self.label("저장", "Save")).on_press(Message::ProjectSave),
                button(self.label("취소", "Cancel"))
                    .style(button::secondary)
                    .on_press(Message::Page(Page::Chat))
            ]
            .spacing(8)
        ]
        .spacing(12);
        if let Some(error) = &self.core.project_error {
            content = content.push(
                text(self.core.settings.language.message(error).into_owned()).style(text::danger),
            );
        }
        scrollable(container(content).max_width(740).width(Fill)).into()
    }

    fn global_view(&self) -> Element<'_, Message> {
        let g = &self.core.global;
        let mut tabs = row![].spacing(5);
        for (tab, (ko, en)) in [
            ("공통 지침", "Instructions"),
            ("모델·권한", "Defaults"),
            ("스킬", "Skills"),
            ("실행·MCP", "CLI / MCP"),
            ("고급", "Advanced"),
        ]
        .iter()
        .enumerate()
        {
            tabs = tabs.push(
                button(text(self.label(ko, en)).size(12))
                    .style(if g.tab == tab {
                        button::secondary
                    } else {
                        button::text
                    })
                    .on_press(Message::GlobalTab(tab)),
            );
        }
        let mut content = column![
            self.heading(self.label("전역 설정", "Global settings")),
            tabs.wrap()
        ]
        .spacing(14);
        if !g.message.is_empty() {
            content = content
                .push(text(self.core.settings.language.message(&g.message).into_owned()).size(12));
        }
        if g.pending.is_some() {
            return content
                .push(text(self.label("설정 처리 중…", "Loading / saving…")))
                .into();
        }
        let Some(d) = &g.docs else {
            return content
                .push(button(self.label("불러오기", "Load")).on_press(Message::GlobalReload))
                .into();
        };
        let idle = self.global_idle();
        if !idle {
            content = content.push(
                text(self.label(
                    "작업 중에는 전역 설정이 잠깁니다.",
                    "Global settings are locked while tasks are running.",
                ))
                .size(12)
                .style(text::secondary),
            );
        }
        if !d.config_valid {
            content = content.push(
                text(self.label(
                    "TOML 오류가 있습니다. 고급 탭에서 수정하세요.",
                    "Invalid TOML. Fix it in Advanced.",
                ))
                .style(text::danger),
            );
        }
        match g.tab {
            0 | 4 => {
                let config = g.tab == 4;
                let file = if config { &d.config } else { &d.agents };
                content = content
                    .push(
                        text(file.path.to_string_lossy().into_owned())
                            .size(11)
                            .font(iced::Font::MONOSPACE),
                    )
                    .push(
                        text_editor(&self.global_editor)
                            .on_action(Message::GlobalEdit)
                            .height(320)
                            .size(13),
                    )
                    .push(
                        button(self.label("저장", "Save")).on_press_maybe(
                            (idle && file.dirty() && (!config || d.config_valid))
                                .then_some(Message::GlobalSave(config)),
                        ),
                    );
            }
            1 => {
                for (key, ko, en) in [
                    ("model", "기본 모델", "Default model"),
                    ("model_reasoning_effort", "추론 강도", "Reasoning effort"),
                    ("approval_policy", "승인 정책", "Approval policy"),
                    ("sandbox_mode", "실행 권한", "Sandbox mode"),
                ] {
                    let value = d
                        .parsed
                        .get(key)
                        .and_then(toml_edit::Item::as_str)
                        .unwrap_or("");
                    content = content.push(
                        row![
                            text(self.label(ko, en)).width(140).size(13),
                            text_input(self.label("Codex 기본값", "Codex default"), value)
                                .on_input_maybe(
                                    (idle && d.config_valid)
                                        .then_some(move |v| Message::GlobalField(key.into(), v))
                                )
                        ]
                        .spacing(12)
                        .align_y(iced::Center),
                    );
                }
                content = content.push(
                    button(self.label("기본값 저장", "Save defaults")).on_press_maybe(
                        (idle && d.config_valid && d.config.dirty())
                            .then_some(Message::GlobalSave(true)),
                    ),
                );
            }
            2 => {
                content = content.push(
                    text_input(self.label("스킬 검색", "Search skills"), &g.filter)
                        .on_input(Message::SkillFilter),
                );
                content = content.push(
                    button(self.label("스킬 목록 새로고침", "Refresh skills")).on_press_maybe(
                        (self.core.ready && !d.config.dirty()).then_some(Message::RefreshSkills),
                    ),
                );
                for skill in g.skills.iter().filter(|s| {
                    string(s, "name")
                        .to_lowercase()
                        .contains(&g.filter.to_lowercase())
                }) {
                    let path = string(skill, "path").to_owned();
                    content = content
                        .push(
                            checkbox(skill["enabled"].as_bool().unwrap_or(true))
                                .label(string(skill, "name"))
                                .text_size(13)
                                .on_toggle_maybe(
                                    (idle && self.core.ready && !d.config.dirty())
                                        .then_some(move |v| Message::Skill(path.clone(), v)),
                                ),
                        )
                        .push(
                            text(string(skill, "description"))
                                .size(11)
                                .style(text::secondary),
                        );
                }
            }
            _ => {
                content = content
                    .push(text(self.label("Codex 실행 파일", "Codex executable")).size(13))
                    .push(
                        text_input(
                            "codex",
                            g.binary_draft.as_deref().unwrap_or(&self.core.binary),
                        )
                        .on_input(Message::Binary),
                    )
                    .push(
                        button(self.label("실행 경로 저장", "Save executable"))
                            .on_press_maybe(idle.then_some(Message::SaveBinary)),
                    )
                    .push(widget::rule::horizontal(1));
                if let Some(servers) = d
                    .parsed
                    .get("mcp_servers")
                    .and_then(toml_edit::Item::as_table_like)
                {
                    for (name, item) in servers.iter() {
                        let key = name.to_owned();
                        content = content.push(
                            row![
                                checkbox(
                                    item.get("enabled")
                                        .and_then(toml_edit::Item::as_bool)
                                        .unwrap_or(true)
                                )
                                .label(name)
                                .on_toggle_maybe(
                                    idle.then_some(move |v| Message::McpToggle(key.clone(), v))
                                ),
                                self.small_button(
                                    self.label("편집", "Edit"),
                                    Message::McpEdit(name.into())
                                )
                            ]
                            .spacing(8)
                            .align_y(iced::Center),
                        );
                    }
                }
                content = content
                    .push(
                        text(self.label("MCP 서버 추가 / 편집", "Add / edit MCP server")).size(15),
                    )
                    .push(
                        text_input(self.label("서버 이름", "Server name"), &g.mcp_name)
                            .on_input(|v| Message::McpField(0, v)),
                    )
                    .push(
                        checkbox(g.mcp_http)
                            .label("HTTP URL")
                            .on_toggle(Message::McpHttp),
                    );
                if g.mcp_http {
                    content = content.push(
                        text_input("https://…", &g.mcp_url).on_input(|v| Message::McpField(3, v)),
                    );
                } else {
                    content = content
                        .push(
                            text_input(self.label("실행 파일", "Command"), &g.mcp_command)
                                .on_input(|v| Message::McpField(1, v)),
                        )
                        .push(
                            column![
                                text(self.label("인수 · 한 줄에 하나", "Arguments · one per line"))
                                    .size(12),
                                text_editor(&self.mcp_arguments)
                                    .on_action(Message::McpArguments)
                                    .height(90)
                                    .size(13)
                            ]
                            .spacing(6),
                        );
                }
                content = content.push(
                    row![
                        button(self.label("설정 반영", "Apply to draft"))
                            .on_press_maybe((idle && d.config_valid).then_some(Message::McpApply)),
                        button(self.label("MCP 설정 저장", "Save MCP config")).on_press_maybe(
                            (idle && d.config_valid && d.config.dirty())
                                .then_some(Message::GlobalSave(true))
                        )
                    ]
                    .spacing(8),
                );
            }
        }
        content = content.push(widget::rule::horizontal(1)).push(
            button(self.label("파일 다시 불러오기", "Reload files"))
                .style(button::secondary)
                .on_press_maybe(
                    (!d.config.dirty() && !d.agents.dirty()).then_some(Message::GlobalReload),
                ),
        );
        scrollable(container(content).max_width(800).width(Fill)).into()
    }

    fn usage_records(&self) -> Vec<crate::usage::Record> {
        let mut records: Vec<_> = self.core.meter.records.iter().cloned().collect();
        for meter in
            std::iter::once(&self.core.meter).chain(self.core.sessions.iter().map(|s| &s.meter))
        {
            if let Some(record) = meter.current() {
                let mut record = record.clone();
                record.elapsed_secs = meter.elapsed_secs();
                records.retain(|r| r.thread != record.thread || r.turn != record.turn);
                records.push(record);
            }
        }
        records.sort_by_key(|r| std::cmp::Reverse(r.started_at));
        records.truncate(crate::usage::KEEP);
        records
    }

    fn usage_summary(&self) -> Element<'_, Message> {
        let c = &self.core;
        let colors = c.settings.theme.colors();
        let mut content = column![
            row![
                text(self.label("계정 한도", "Account limits")).size(15),
                space().width(Fill),
                self.small_button(self.label("새로고침", "Refresh"), Message::RefreshUsage)
            ]
            .align_y(iced::Center)
        ]
        .spacing(12);
        let mut windows = 0;
        for bucket in c.rate_limits.buckets.values() {
            for window in [&bucket.primary, &bucket.secondary].into_iter().flatten() {
                let period = limit_period(window.window_duration_mins, c.settings.language);
                let remaining = (100. - window.used_percent).clamp(0., 100.);
                let mut label = column![
                    row![
                        text(format!("{} · {period}", bucket.label)).size(13),
                        space().width(Fill),
                        text(format!("{remaining:.1}% {}", self.label("남음", "left"))).size(13)
                    ]
                    .spacing(10),
                    widget::progress_bar(0.0..=100.0, remaining as f32).girth(4)
                ]
                .spacing(6);
                if let Some(reset) = window.resets_at {
                    label = label.push(
                        text(reset_countdown(reset, c.settings.language))
                            .size(12)
                            .color(colors.muted),
                    );
                }
                content = content.push(label);
                windows += 1;
            }
        }
        if windows == 0 {
            content = content.push(
                text(self.label(
                    if c.pending.values().any(|p| matches!(p, Pending::RateLimits)) {
                        "한도 조회 중…"
                    } else {
                        "서버에서 제공한 한도 정보가 없습니다."
                    },
                    if c.pending.values().any(|p| matches!(p, Pending::RateLimits)) {
                        "Loading limits…"
                    } else {
                        "No account limits provided by the server."
                    },
                ))
                .size(13)
                .color(colors.muted),
            );
        }
        if let Some(error) = &c.rate_limits.error {
            content = content.push(
                text(format!(
                    "{}: {error}",
                    self.label("조회 실패", "Could not refresh")
                ))
                .size(12)
                .color(colors.muted),
            );
        }
        if let Some((used, limit)) = c.meter.context {
            content = content.push(row![
                text(self.label("컨텍스트", "Context")).size(13),
                space().width(Fill),
                text(format!(
                    "{} / {}",
                    crate::usage::number(used),
                    crate::usage::number(limit)
                ))
                .size(13)
            ]);
        }
        if let Some(record) = c.meter.current() {
            content = content.push(widget::rule::horizontal(1)).push(
                text(self.label(
                    if record.status == "inProgress" {
                        "현재 작업"
                    } else {
                        "최근 작업"
                    },
                    if record.status == "inProgress" {
                        "Current turn"
                    } else {
                        "Latest turn"
                    },
                ))
                .size(14),
            );
            if let Some(tokens) = &record.tokens {
                let values = [
                    (self.label("입력", "Input"), tokens.input_tokens),
                    (self.label("캐시", "Cached"), tokens.cached_input_tokens),
                    (self.label("출력", "Output"), tokens.output_tokens),
                    (
                        self.label("추론", "Reasoning"),
                        tokens.reasoning_output_tokens,
                    ),
                ];
                let mut stats = row![].spacing(18);
                for (label, count) in values {
                    stats = stats.push(
                        column![
                            text(label).size(12).color(colors.muted),
                            text(crate::usage::number(count)).size(15)
                        ]
                        .spacing(4),
                    );
                }
                content = content.push(stats.wrap());
            } else {
                content = content.push(
                    text(self.label("서버 사용량 수신 대기", "Waiting for server usage"))
                        .size(13)
                        .color(colors.muted),
                );
            }
            if record.partial {
                content = content.push(
                    text(self.label("대화 재개 후 관측분", "Observed since resuming"))
                        .size(12)
                        .color(colors.muted),
                );
            }
        }
        content.into()
    }

    fn usage_view(&self) -> Element<'_, Message> {
        let mut content = column![
            row![
                self.heading(self.label("사용량", "Usage")),
                space().width(Fill),
                self.small_button(self.label("JSON 복사", "Copy JSON"), Message::CopyUsage)
            ]
            .align_y(iced::Center),
            self.usage_summary(),
            widget::rule::horizontal(1)
        ]
        .spacing(14);
        if let Some(error) = &self.core.usage_error {
            content = content.push(text(error).style(text::danger));
        }
        for record in self.usage_records() {
            content = content.push(usage_row(&record));
        }
        scrollable(container(content).max_width(740).width(Fill)).into()
    }
    fn resources_view(&self) -> Element<'_, Message> {
        let mut content = column![
            self.heading(self.label("리소스 진단", "Resources")),
            button(self.label("새로고침", "Refresh")).on_press(Message::RefreshResources),
            row![
                text("PID").width(70),
                text(self.label("프로세스", "Process")).width(Fill),
                text("CPU").width(80),
                text("RSS").width(90)
            ]
            .spacing(8),
            widget::rule::horizontal(1)
        ]
        .spacing(12);
        if let Some(monitor) = &self.monitor {
            for p in &monitor.rows {
                content = content.push(
                    row![
                        text(p.pid).width(70),
                        text(format!("{} · {}", p.role, p.name)).width(Fill),
                        text(format!("{:.1}%", p.cpu)).width(80),
                        text(format!("{:.1} MiB", p.memory as f64 / 1048576.0)).width(90)
                    ]
                    .spacing(8),
                );
            }
        }
        if let Some(error) = &self.core.desktop.last_error {
            content = content.push(
                text(format!("Desktop: {error}"))
                    .size(12)
                    .style(text::danger),
            );
        }
        for line in &self.core.diagnostics {
            content = content.push(text(line).font(iced::Font::MONOSPACE).size(11));
        }
        scrollable(content).into()
    }
}

fn limit_period(minutes: Option<u64>, language: Language) -> String {
    let ko = language == Language::Korean;
    match minutes {
        Some(m) if m > 0 && m % 1440 == 0 => format!("{}{}", m / 1440, if ko { "일" } else { "d" }),
        Some(m) if m > 0 && m % 60 == 0 => format!("{}{}", m / 60, if ko { "시간" } else { "h" }),
        Some(m) if m > 0 => format!("{m}{}", if ko { "분" } else { "m" }),
        _ => if ko { "한도" } else { "Limit" }.into(),
    }
}

fn reset_countdown(reset: u64, language: Language) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let minutes = reset.saturating_sub(now).div_ceil(60);
    let ko = language == Language::Korean;
    if minutes == 0 {
        return if ko {
            "갱신 대기"
        } else {
            "Awaiting refresh"
        }
        .into();
    }
    let duration = if minutes >= 1440 {
        format!(
            "{}{} {}{}",
            minutes / 1440,
            if ko { "일" } else { "d" },
            minutes % 1440 / 60,
            if ko { "시간" } else { "h" }
        )
    } else if minutes >= 60 {
        format!(
            "{}{} {}{}",
            minutes / 60,
            if ko { "시간" } else { "h" },
            minutes % 60,
            if ko { "분" } else { "m" }
        )
    } else {
        format!("{minutes}{}", if ko { "분" } else { "m" })
    };
    if ko {
        format!("{duration} 후 초기화")
    } else {
        format!("Resets in {duration}")
    }
}

fn usage_row(record: &crate::usage::Record) -> Element<'static, Message> {
    let summary = record
        .tokens
        .as_ref()
        .map(|t| {
            format!(
                "in {} · cached {} · out {} · reasoning {}",
                crate::usage::number(t.input_tokens),
                crate::usage::number(t.cached_input_tokens),
                crate::usage::number(t.output_tokens),
                crate::usage::number(t.reasoning_output_tokens)
            )
        })
        .unwrap_or_else(|| "—".into());
    column![
        text(format!(
            "{} · {} · {} · {}s{}",
            record.project,
            record.model,
            record.status,
            record.elapsed_secs,
            if record.partial { " · partial" } else { "" }
        ))
        .size(13),
        text(summary).size(12).font(iced::Font::MONOSPACE),
        widget::rule::horizontal(1)
    ]
    .spacing(6)
    .into()
}

fn populate_preview(core: &mut MiniApp) {
    core.settings.projects[0].name = "codex-mini".into();
    core.settings.model = "gpt-6".into();
    core.settings.effort = "medium".into();
    core.models = vec![
        json!({"model":"gpt-6","displayName":"GPT-6","supportedReasoningEfforts":[{"reasoningEffort":"low"},{"reasoningEffort":"medium"},{"reasoningEffort":"high"}]}),
    ];
    core.thread_id = Some("preview".into());
    core.status = "완료".into();
    core.ready = true;
    core.settings.remember("preview", "프로젝트 실행 흐름 정리");
    core.settings.remember("preview-2", "테스트 실패 원인 확인");
    core.settings.remember("preview-3", "대화 기록 불러오기");
    core.transcript.put(
        "user",
        "나",
        "이 프로젝트의 실행 흐름을 간단히 정리해줘.",
        false,
        false,
    );
    core.transcript.put("answer","CODEX","프로젝트는 **화면, 실행 제어, CLI 통신**으로 나뉩니다.\n\n1. 화면에서 메시지를 보내면 실행 제어 계층이 대화와 작업 상태를 갱신합니다.\n2. 통신 계층이 Codex CLI에 요청을 전달합니다.\n3. 응답이 도착하면 대화 내용과 실행 상태가 화면에 반영됩니다.\n\n```rust\nlet backend = Backend::start(binary, cwd, wake)?;\n```\n\n파일 변경과 명령 실행에 승인이 필요하면 요청 내용을 먼저 확인할 수 있습니다.",false,false);
    core.transcript.put(
        "command",
        "명령 · completed",
        "cargo test --locked\n\nrunning 51 tests\ntest result: ok",
        true,
        false,
    );
    core.prompt = String::new();
    if std::env::args().any(|a| matches!(a.as_str(), "--usage" | "--usage-popup")) {
        let reset = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + 7200;
        core.rate_limits.update(&json!({"rateLimits":{"limitId":"codex","primary":{"usedPercent":25,"windowDurationMins":300,"resetsAt":reset},"secondary":{"usedPercent":40,"windowDurationMins":10080,"resetsAt":reset+86400}}}), true);
        core.meter.attach("preview", true);
        core.meter.begin("codex-mini", "gpt-6", "medium", "base");
        core.meter.turn("preview-turn");
        core.meter.update(&json!({"threadId":"preview","turnId":"preview-turn","tokenUsage":{"total":{"inputTokens":24000,"cachedInputTokens":16000,"outputTokens":1200,"reasoningOutputTokens":900,"totalTokens":25200},"last":{"inputTokens":24000,"outputTokens":1200,"totalTokens":25200},"modelContextWindow":128000}}));
        core.meter.finish("completed");
    }
    if std::env::args().any(|a| a == "--empty") {
        core.transcript = Default::default();
        core.thread_id = None;
    }
    if std::env::args().any(|a| a == "--login") {
        core.transcript = Default::default();
        core.thread_id = None;
        core.needs_auth = true;
        core.account = "로그인 필요".into();
        core.settings.model.clear();
        core.models.clear();
    }
    if std::env::args().any(|a| a == "--setup") {
        core.transcript = Default::default();
        core.thread_id = None;
        core.ready = false;
        core.settings.model.clear();
        core.models.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> App {
        let wake = Wake::default();
        let core = MiniApp::new_with_services(&wake, Settings::default(), "unused".into(), false);
        App {
            popup: None,
            core,
            wake,
            window: window::Id::unique(),
            page: Page::Chat,
            sidebar: true,
            width: 940.,
            height: 700.,
            composer: text_editor::Content::new(),
            instructions: text_editor::Content::new(),
            global_editor: text_editor::Content::new(),
            mcp_arguments: text_editor::Content::new(),
            mcp_exclusions: text_editor::Content::new(),
            exclusions_source: String::new(),
            global_source: String::new(),
            expanded: HashSet::new(),
            raw: HashMap::new(),
            markdown: HashMap::new(),
            visible: 40,
            follow_bottom: true,
            chat_options: None,
            monitor: None,
            scroll: widget::Id::unique(),
            composer_id: widget::Id::unique(),
            smoke_output: None,
            benchmark_output: None,
            booted: std::time::Instant::now(),
        }
    }
    fn key(shift: bool, focused: bool) -> text_editor::KeyPress {
        text_editor::KeyPress {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
            modified_key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter),
            physical_key: iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Enter),
            modifiers: if shift {
                iced::keyboard::Modifiers::SHIFT
            } else {
                iced::keyboard::Modifiers::empty()
            },
            text: None,
            status: if focused {
                text_editor::Status::Focused { is_hovered: false }
            } else {
                text_editor::Status::Active
            },
        }
    }
    #[test]
    fn enter_sends_only_in_the_focused_composer_and_shift_enter_is_a_newline() {
        assert!(matches!(
            composer_binding(key(false, true)),
            Some(text_editor::Binding::Custom(Message::Send))
        ));
        assert!(matches!(
            composer_binding(key(true, true)),
            Some(text_editor::Binding::Enter)
        ));
        assert!(composer_binding(key(false, false)).is_none());
    }
    #[test]
    fn composing_text_cannot_be_sent() {
        let mut app = fixture();
        app.core.ready = true;
        app.core.prompt = "한글".into();
        let _ = app.update(Message::Composing(true));
        let _ = app.update(Message::Send);
        assert_eq!(app.core.prompt, "한글");
        assert!(app.core.pending.is_empty());
        let _ = app.update(Message::Composing(false));
        assert!(!app.core.prompt_composing);
    }
    #[test]
    fn cancelled_queue_restores_text_without_overwriting_newer_drafts() {
        let mut app = fixture();
        app.core.prompt = "new draft".into();
        app.core.queued_job = Some(crate::controller::QueuedJob {
            order: 1,
            prompt: "queued".into(),
            model: String::new(),
            effort: String::new(),
        });
        let _ = app.update(Message::CancelQueued(None));
        assert!(app.core.queued_job.is_none());
        assert_eq!(app.core.prompt, "new draft\n\nqueued");
        assert_eq!(app.composer.text(), app.core.prompt);
    }
    #[test]
    fn raw_output_allows_selection_actions_but_cannot_be_edited() {
        let mut app = fixture();
        app.core
            .transcript
            .put("m", "CODEX", "original 한글", false, false);
        let _ = app.update(Message::Raw("m".into()));
        let _ = app.update(Message::SelectText(
            "m".into(),
            text_editor::Action::Edit(text_editor::Edit::Insert('X')),
        ));
        assert_eq!(app.raw["m"].text(), "original 한글");
    }
    #[test]
    fn hidden_window_defers_markdown_parsing_and_can_restore_without_a_tray() {
        let mut app = fixture();
        app.core
            .transcript
            .put("m", "CODEX", "**hello**", false, false);
        app.core.hidden = true;
        app.sync();
        assert!(app.markdown.is_empty());
        let _ = app.update(Message::Window(app.window, window::Event::Focused));
        assert!(!app.core.hidden);
        assert_eq!(app.markdown.len(), 1);
    }
    #[test]
    fn quit_confirms_for_background_and_queued_work() {
        let mut app = fixture();
        app.core.queued_job = Some(crate::controller::QueuedJob {
            order: 1,
            prompt: "queued".into(),
            model: String::new(),
            effort: String::new(),
        });
        let _ = app.quit();
        assert!(app.core.show_exit_confirmation);
        assert!(!app.core.exiting);
    }
    #[test]
    fn missing_permission_details_and_incomplete_answers_cannot_be_accepted() {
        let mut app = fixture();
        app.core.server_request(json!({"id":1,"method":"item/permissions/requestApproval","params":{"reason":"required"}}));
        let _ = app.update(Message::Approve(true));
        assert_eq!(app.core.approvals.len(), 1);
        assert!(app.core.error.is_none());
        app.core.approvals.clear();
        app.core.server_request(json!({"id":2,"method":"item/tool/requestUserInput","params":{"questions":[{"id":"q","question":"choose"}]}}));
        assert!(!answers_complete(app.core.approvals.front().unwrap()));
        let _ = app.update(Message::Answer("q".into(), "answer".into()));
        assert!(answers_complete(app.core.approvals.front().unwrap()));
    }
    #[test]
    fn view_builds_in_both_themes_at_the_minimum_size() {
        let mut app = fixture();
        populate_preview(&mut app.core);
        app.width = 560.;
        app.sidebar = false;
        for theme in [Theme::Dark, Theme::Light] {
            app.core.settings.theme = theme;
            app.sync();
            let _ = app.view(app.window);
        }
    }

    #[test]
    fn project_can_exclude_mcp_servers_not_listed_in_global_config() {
        let mut app = fixture();
        app.mcp_exclusions = text_editor::Content::with_text("project-server\nsecond_server");
        let _ = app.update(Message::SaveMcpExclusions);
        assert_eq!(
            app.core.settings.active().unwrap().efficiency.disabled_mcp,
            vec!["project-server", "second_server"]
        );
    }

    #[test]
    fn escape_closes_the_menu_before_leaving_the_current_page() {
        let mut app = fixture();
        app.page = Page::Settings;
        let _ = app.update(Message::Menu(Popup::Composer));
        let _ = app.update(Message::Dismiss);
        assert_eq!(app.popup, None);
        assert_eq!(app.page, Page::Settings);
        let _ = app.update(Message::Dismiss);
        assert_eq!(app.page, Page::Chat);
    }

    #[test]
    fn narrowing_the_window_closes_the_sidebar_without_losing_the_draft() {
        let mut app = fixture();
        app.core.prompt = "draft 한글".into();
        let _ = app.update(Message::Window(
            app.window,
            window::Event::Resized(iced::Size::new(560., 440.)),
        ));
        assert!(!app.sidebar);
        assert_eq!(app.core.prompt, "draft 한글");
        let _ = app.update(Message::Sidebar);
        assert!(app.sidebar);
        let _ = app.view(app.window);
        let _ = app.update(Message::Dismiss);
        assert!(!app.sidebar);
    }
}
