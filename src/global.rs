use eframe::egui::{self, RichText};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    sync::mpsc,
    time::{SystemTime, UNIX_EPOCH},
};
use toml_edit::{DocumentMut, Item, Table, value};

#[derive(Clone)]
pub struct FileDoc {
    pub path: PathBuf,
    pub text: String,
    original: Option<String>,
}
impl FileDoc {
    fn read(path: PathBuf) -> Result<Self, String> {
        let original = match fs::read_to_string(&path) {
            Ok(s) if s.len() <= 1024 * 1024 => Some(s),
            Ok(_) => return Err("설정 파일이 1 MiB를 넘습니다.".into()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        Ok(Self {
            text: original.clone().unwrap_or_default(),
            path,
            original,
        })
    }
    fn dirty(&self) -> bool {
        self.original.as_deref().unwrap_or("") != self.text
    }
    fn save(mut self, toml: bool) -> Result<Self, String> {
        if self.text.len() > 1024 * 1024 {
            return Err("설정 파일은 1 MiB 이하로 저장하세요.".into());
        }
        if toml {
            self.text
                .parse::<DocumentMut>()
                .map_err(|e| format!("TOML 문법 오류: {e}"))?;
        }
        let current = Self::read(self.path.clone())?;
        if current.original != self.original {
            return Err("다른 프로그램에서 파일을 변경했습니다. 다시 불러온 뒤 수정하세요.".into());
        }
        fs::create_dir_all(self.path.parent().ok_or("설정 경로 오류")?)
            .map_err(|e| e.to_string())?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        if self.path.exists() {
            fs::copy(
                &self.path,
                self.path.with_extension(format!("mini-backup-{stamp}")),
            )
            .map_err(|e| e.to_string())?;
        }
        let tmp = self.path.with_extension(format!("mini-{stamp}.tmp"));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp).map_err(|e| e.to_string())?;
        use std::io::Write;
        file.write_all(self.text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        if let Ok(meta) = fs::metadata(&self.path) {
            fs::set_permissions(&tmp, meta.permissions()).map_err(|e| e.to_string())?;
        }
        // Check again after preparing the replacement; never silently overwrite detected edits.
        if Self::read(self.path.clone())?.original != self.original {
            let _ = fs::remove_file(tmp);
            return Err("저장 직전에 파일이 변경되었습니다. 다시 불러오세요.".into());
        }
        fs::rename(tmp, &self.path).map_err(|e| e.to_string())?;
        self.original = Some(self.text.clone());
        Ok(self)
    }
}
struct Documents {
    agents: FileDoc,
    config: FileDoc,
    parsed: DocumentMut,
    config_valid: bool,
}
enum Job {
    Load(Box<Documents>),
    Saved(FileDoc, bool),
    ConfigLoaded(FileDoc, DocumentMut),
}
#[derive(Default)]
pub struct GlobalUi {
    pub open: bool,
    tab: usize,
    docs: Option<Documents>,
    pending: Option<mpsc::Receiver<Result<Job, String>>>,
    pub message: String,
    pub skills: Vec<Value>,
    pub skills_loaded: bool,
    filter: String,
    pub changed: bool,
    mcp_name: String,
    mcp_command: String,
    mcp_args: String,
    mcp_url: String,
    mcp_http: bool,
    mcp_editor_open: bool,
    binary_draft: Option<String>,
}
pub enum Action {
    RefreshSkills,
    Skill(String, bool),
    Binary(String),
}
fn codex_home() -> Result<PathBuf, String> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".codex")))
        .ok_or_else(|| "Codex 설정 폴더를 찾을 수 없습니다.".into())
}
impl GlobalUi {
    fn job(
        &mut self,
        ctx: &egui::Context,
        f: impl FnOnce() -> Result<Job, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(f());
            ctx.request_repaint();
        });
    }
    pub fn load(&mut self, ctx: &egui::Context) {
        self.message.clear();
        self.job(ctx, || {
            let home = codex_home()?;
            let override_path = home.join("AGENTS.override.md");
            let agents_path = if fs::metadata(&override_path).is_ok_and(|m| m.len() > 0) {
                override_path
            } else {
                home.join("AGENTS.md")
            };
            let agents = FileDoc::read(agents_path)?;
            let config = FileDoc::read(home.join("config.toml"))?;
            let parsed = config.text.parse::<DocumentMut>();
            let config_valid = parsed.is_ok();
            let parsed = parsed.unwrap_or_default();
            Ok(Job::Load(Box::new(Documents {
                agents,
                config,
                parsed,
                config_valid,
            })))
        });
    }
    pub fn poll(&mut self) {
        if let Some(result) = self.pending.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.pending = None;
            match result {
                Ok(Job::Load(d)) => self.docs = Some(*d),
                Ok(Job::ConfigLoaded(file, parsed)) => {
                    if let Some(d) = &mut self.docs {
                        d.config = file;
                        d.parsed = parsed;
                        d.config_valid = true;
                    }
                }
                Ok(Job::Saved(file, config)) => {
                    if let Some(d) = &mut self.docs {
                        if config {
                            d.config = file;
                        } else {
                            d.agents = file;
                        }
                    }
                    self.changed = true;
                    self.message =
                        "저장했습니다. 연결을 종료한 뒤 다시 연결하면 적용됩니다.".into();
                }
                Err(e) => self.message = e,
            }
        }
    }
    pub fn refresh_config(&mut self, ctx: &egui::Context) {
        self.job(ctx, || {
            let file = FileDoc::read(codex_home()?.join("config.toml"))?;
            let parsed = file
                .text
                .parse::<DocumentMut>()
                .map_err(|e| e.to_string())?;
            Ok(Job::ConfigLoaded(file, parsed))
        });
    }
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        ready: bool,
        idle: bool,
        binary: &str,
    ) -> Option<Action> {
        self.poll();
        if self.binary_draft.is_none() {
            self.binary_draft = Some(binary.to_owned());
        }
        if !self.open {
            return None;
        }
        if self.docs.is_none() && self.pending.is_none() && self.message.is_empty() {
            self.load(ctx);
        }
        let mut action = None;
        let mut open = true;
        let mut save = None;
        let mut reload = false;
        egui::Window::new("전역 설정").open(&mut open).collapsible(false).resizable(false)
            .anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO)
            .fixed_size(egui::vec2((ctx.content_rect().width()-32.0).min(620.0),(ctx.content_rect().height()-60.0).min(440.0)))
            .vscroll(true).show(ctx,|ui| {
                ui.horizontal_wrapped(|ui| {
                    for (n,label) in ["공통 지침","모델·권한","스킬","실행·MCP","고급"].iter().enumerate() {ui.selectable_value(&mut self.tab,n,*label);}
                });
                ui.separator();
                if !self.message.is_empty(){ui.label(&self.message);}
                if self.pending.is_some(){ui.label("설정 처리 중…");return;}
                let Some(d)=&mut self.docs else {if ui.button("다시 불러오기").clicked(){reload=true;}return;};
                if !d.config_valid { ui.label("config.toml 문법 오류가 있습니다. 고급 탭에서 수정하세요."); }
                ui.add_enabled_ui(idle && (d.config_valid || self.tab == 0 || self.tab == 4),|ui| {
                    match self.tab {
                        0=>{
                            ui.label("모든 프로젝트에 적용되는 공통 지침");
                            ui.add(egui::Label::new(d.agents.path.display().to_string()).truncate()).on_hover_text(d.agents.path.display().to_string());
                            ui.add(egui::TextEdit::multiline(&mut d.agents.text).desired_rows(12).desired_width(f32::INFINITY).char_limit(128*1024));
                            if ui.add_enabled(d.agents.dirty(),egui::Button::new("공통 지침 저장")).clicked(){save=Some((d.agents.clone(),false));}
                        }
                        1=>{
                            ui.label("Codex 사용자 기본값 · 프로젝트 설정이나 관리 정책이 우선할 수 있습니다.");
                            let mut changed=false;
                            egui::Grid::new("global_defaults").num_columns(2).spacing([12.0,6.0]).show(ui,|ui| {
                                for (key,label,choices) in [
                                    ("model","기본 모델",&[][..]),
                                    ("model_reasoning_effort","추론 강도",&["minimal","low","medium","high","xhigh"]),
                                    ("approval_policy","승인 정책",&["untrusted","on-request","never"]),
                                    ("sandbox_mode","실행 권한",&["read-only","workspace-write","danger-full-access"]),
                                ] {
                                    ui.label(label);let mut text=d.parsed.get(key).and_then(Item::as_str).unwrap_or("").to_owned();let before=text.clone();
                                    if choices.is_empty(){ui.add(egui::TextEdit::singleline(&mut text).hint_text("Codex 기본값").desired_width(200.0));}
                                    else {egui::ComboBox::from_id_salt(key).width(200.0).selected_text(if text.is_empty(){"Codex 기본값"}else{&text}).show_ui(ui,|ui|{ui.selectable_value(&mut text,String::new(),"Codex 기본값");for choice in choices{ui.selectable_value(&mut text,(*choice).into(),*choice);}});}
                                    if text!=before {if text.is_empty(){d.parsed.remove(key);}else{d.parsed[key]=value(text);}changed=true;}
                                    ui.end_row();
                                }
                            });
                            ui.small("read-only: 읽기 전용 · workspace-write: 작업 폴더 편집 · danger-full-access: 제한 없음");
                            ui.small("untrusted: 신뢰되지 않은 작업 확인 · on-request: 필요할 때 확인 · never: 승인 질문 없음");
                            if changed{d.config.text=d.parsed.to_string();}
                            if ui.add_enabled(d.config.dirty(),egui::Button::new("기본값 저장")).clicked(){save=Some((d.config.clone(),true));}
                        }
                        2=>{
                            ui.label("Codex가 인식하는 스킬 · 켜기/끄기는 전역 사용자 설정에 저장됩니다.");
                            if !ready {ui.label("먼저 메인 화면에서 Codex에 연결하세요.");}
                            if ui.add_enabled(ready && !d.config.dirty(),egui::Button::new("스킬 목록 새로고침")).clicked(){action=Some(Action::RefreshSkills);}
                            ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("스킬 검색"));
                            let filter=self.filter.to_lowercase();
                            for skill in self.skills.iter().filter(|s|s["name"].as_str().unwrap_or("").to_lowercase().contains(&filter)) {
                                let name=skill["name"].as_str().unwrap_or("");let path=skill["path"].as_str().unwrap_or("");let mut enabled=skill["enabled"].as_bool().unwrap_or(true);
                                ui.push_id(path,|ui| {
                                    if ui.add_enabled(ready && !d.config.dirty(),egui::Checkbox::new(&mut enabled,name)).on_hover_text(path).changed(){action=Some(Action::Skill(path.into(),enabled));}
                                    ui.label(RichText::new(skill["description"].as_str().unwrap_or("")).small());
                                });
                            }
                            if d.config.dirty(){ui.label("스킬을 변경하려면 다른 전역 설정을 먼저 저장하세요.");}
                        }
                        3=>{
                            ui.label("Codex 실행 파일");
                            let draft=self.binary_draft.as_mut().unwrap();
                            ui.text_edit_singleline(draft);
                            if ui.add_enabled(!draft.trim().is_empty() && draft != binary,egui::Button::new("실행 경로 저장")).clicked(){action=Some(Action::Binary(draft.trim().into()));self.changed=true;}
                            ui.small("실행 경로는 Mini에 저장됩니다. MCP 설정은 Codex config.toml에 저장됩니다.");
                            ui.separator();ui.strong("MCP 서버");
                            let mut toggle=None;
                            if let Some(servers)=d.parsed.get("mcp_servers").and_then(Item::as_table_like){
                                for (name,item) in servers.iter(){
                                    let mut enabled=item.get("enabled").and_then(Item::as_bool).unwrap_or(true);
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut enabled,name).changed(){toggle=Some((name.to_owned(),enabled));}
                                        if ui.small_button("편집").clicked(){
                                            self.mcp_editor_open=true;self.mcp_name=name.into();self.mcp_command=item.get("command").and_then(Item::as_str).unwrap_or("").into();
                                            self.mcp_url=item.get("url").and_then(Item::as_str).unwrap_or("").into();self.mcp_http = !self.mcp_url.is_empty();
                                            self.mcp_args=item.get("args").and_then(Item::as_array).map(|a|a.iter().filter_map(|s|s.as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default();
                                        }
                                    });
                                }
                            }
                            if let Some((name,enabled))=toggle {d.parsed["mcp_servers"][&name]["enabled"]=value(enabled);d.config.text=d.parsed.to_string();}
                            ui.toggle_value(&mut self.mcp_editor_open,"서버 추가 / 편집");
                            if self.mcp_editor_open { ui.scope(|ui| {
                                ui.label("서버 이름");ui.text_edit_singleline(&mut self.mcp_name);
                                ui.horizontal(|ui| {ui.selectable_value(&mut self.mcp_http,false,"로컬 명령");ui.selectable_value(&mut self.mcp_http,true,"HTTP URL");});
                                if self.mcp_http{ui.label("서버 URL");ui.text_edit_singleline(&mut self.mcp_url);}else{
                                    ui.label("실행 파일");ui.text_edit_singleline(&mut self.mcp_command);ui.label("인수 · 한 줄에 하나");ui.add(egui::TextEdit::multiline(&mut self.mcp_args).desired_rows(3));
                                }
                                if ui.button("서버 설정 반영").clicked(){
                                    match update_mcp(&mut d.parsed,&self.mcp_name,&self.mcp_command,&self.mcp_args,&self.mcp_url,self.mcp_http){Ok(())=>{d.config.text=d.parsed.to_string();self.message="아래 저장 버튼을 눌러 적용하세요.".into();},Err(e)=>self.message=e}
                                }
                            }); }
                            if ui.add_enabled(d.config.dirty(),egui::Button::new("MCP 설정 저장")).clicked(){save=Some((d.config.clone(),true));}
                            ui.small("환경 변수·인증·프로필 등 추가 옵션은 고급 탭에서 편집할 수 있습니다.");
                        }
                        _=>{
                            ui.add(egui::Label::new(d.config.path.display().to_string()).truncate());
                            if ui.add(egui::TextEdit::multiline(&mut d.config.text).code_editor().desired_rows(14).desired_width(f32::INFINITY).char_limit(1024*1024)).changed(){
                                match d.config.text.parse::<DocumentMut>(){Ok(doc)=>{d.parsed=doc;d.config_valid=true;self.message.clear();},Err(e)=>{d.config_valid=false;self.message=format!("TOML 문법 오류: {e}")}}
                            }
                            if ui.add_enabled(d.config.dirty(),egui::Button::new("config.toml 저장")).clicked(){save=Some((d.config.clone(),true));}
                        }
                    }
                });
                if !idle{ui.label("작업이 끝난 뒤 전역 설정을 변경할 수 있습니다.");}
                ui.separator();
                if ui.add_enabled(!d.agents.dirty() && !d.config.dirty(),egui::Button::new("파일 다시 불러오기")).clicked(){reload=true;}
                ui.small("파일 저장 전 원본을 백업하며 외부 변경을 감지하면 저장을 중단합니다.");
            });
        if let Some((file, is_toml)) = save {
            self.job(ctx, move || {
                file.save(is_toml).map(|f| Job::Saved(f, is_toml))
            });
        }
        if reload {
            self.load(ctx);
        }
        self.open = open;
        action
    }
}
fn update_mcp(
    doc: &mut DocumentMut,
    name: &str,
    command: &str,
    args: &str,
    url: &str,
    http: bool,
) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("서버 이름은 영문·숫자·밑줄·하이픈 1~64자로 입력하세요.".into());
    }
    if (http && !(url.starts_with("https://") || url.starts_with("http://")))
        || (!http && command.trim().is_empty())
    {
        return Err("실행 파일 또는 http(s) URL을 입력하세요.".into());
    }
    let servers = doc
        .entry("mcp_servers")
        .or_insert(Item::Table(Table::new()))
        .as_table_like_mut()
        .ok_or("mcp_servers가 테이블 형식이 아닙니다.")?;
    if !servers.contains_key(name) {
        servers.insert(name, Item::Table(Table::new()));
    }
    let server = servers
        .get_mut(name)
        .and_then(Item::as_table_like_mut)
        .ok_or("서버 설정이 테이블 형식이 아닙니다.")?;
    if http {
        server.remove("command");
        server.remove("args");
        server.insert("url", value(url));
    } else {
        server.remove("url");
        server.insert("command", value(command));
        let mut a = toml_edit::Array::new();
        for arg in args.lines() {
            a.push(arg);
        }
        server.insert("args", value(a));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mcp_updates_preserve_other_settings_and_comments() {
        let mut doc = "# keep this\nmodel = 'example'\n[mcp_servers.local]\ncommand = 'old'\n[mcp_servers.local.env]\nTEST_VALUE = 'keep'\n".parse::<DocumentMut>().unwrap();
        update_mcp(
            &mut doc,
            "local",
            "new",
            "--arg\na value with spaces",
            "",
            false,
        )
        .unwrap();
        assert_eq!(
            doc["mcp_servers"]["local"]["env"]["TEST_VALUE"].as_str(),
            Some("keep")
        );
        assert_eq!(
            doc["mcp_servers"]["local"]["args"][1].as_str(),
            Some("a value with spaces")
        );
        assert!(doc.to_string().contains("# keep this"));
        update_mcp(&mut doc, "local", "", "", "https://example.test/mcp", true).unwrap();
        assert!(doc["mcp_servers"]["local"].get("command").is_none());
        assert!(update_mcp(&mut doc, "bad.name", "x", "", "", false).is_err());
    }
    #[test]
    fn file_save_validates_toml_detects_conflicts_and_backs_up() {
        let dir = std::env::temp_dir().join(format!(
            "mini-global-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "model = 'old'\n").unwrap();
        let mut doc = FileDoc::read(path.clone()).unwrap();
        doc.text = "[invalid".into();
        assert!(doc.clone().save(true).is_err());
        doc.text = "model = 'new'\n".into();
        fs::write(&path, "model = 'external'\n").unwrap();
        assert!(doc.save(true).is_err());
        let mut fresh = FileDoc::read(path.clone()).unwrap();
        fresh.text = "model = 'saved'\n".into();
        fresh.save(true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "model = 'saved'\n");
        assert!(fs::read_dir(&dir).unwrap().any(|p| {
            p.unwrap()
                .file_name()
                .to_string_lossy()
                .contains("mini-backup")
        }));
        fs::remove_dir_all(dir).unwrap();
    }
}
