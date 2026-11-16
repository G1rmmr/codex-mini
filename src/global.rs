use crate::runtime::Wake;
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
    pub fn read(path: PathBuf) -> Result<Self, String> {
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
    pub fn dirty(&self) -> bool {
        self.original.as_deref().unwrap_or("") != self.text
    }
    pub fn save(mut self, toml: bool) -> Result<Self, String> {
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
pub struct Documents {
    pub agents: FileDoc,
    pub config: FileDoc,
    pub parsed: DocumentMut,
    pub config_valid: bool,
}
pub enum Job {
    Load(Box<Documents>),
    Saved(FileDoc, bool),
    ConfigLoaded(FileDoc, DocumentMut),
}
#[derive(Default)]
pub struct GlobalUi {
    pub tab: usize,
    pub docs: Option<Documents>,
    pub pending: Option<mpsc::Receiver<Result<Job, String>>>,
    pub message: String,
    pub skills: Vec<Value>,
    pub skills_loaded: bool,
    pub filter: String,
    pub changed: bool,
    pub mcp_name: String,
    pub mcp_command: String,
    pub mcp_args: String,
    pub mcp_url: String,
    pub mcp_http: bool,
    pub binary_draft: Option<String>,
}
fn codex_home() -> Result<PathBuf, String> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| crate::platform::home_dir().map(|p| p.join(".codex")))
        .ok_or_else(|| "Codex 설정 폴더를 찾을 수 없습니다.".into())
}
impl GlobalUi {
    pub fn save_document(&mut self, ctx: &Wake, config: bool) {
        if self.pending.is_some() {
            return;
        }
        if let Some(d) = &self.docs {
            let file = if config {
                d.config.clone()
            } else {
                d.agents.clone()
            };
            self.job(ctx, move || {
                file.save(config).map(|f| Job::Saved(f, config))
            });
        }
    }

    pub fn edit_config(&mut self, text: String) {
        if let Some(d) = &mut self.docs {
            d.config.text = text;
            match d.config.text.parse::<DocumentMut>() {
                Ok(parsed) => {
                    d.parsed = parsed;
                    d.config_valid = true;
                    self.message.clear();
                }
                Err(error) => {
                    d.config_valid = false;
                    self.message = format!("TOML: {error}");
                }
            }
        }
    }
    fn job(&mut self, ctx: &Wake, f: impl FnOnce() -> Result<Job, String> + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        self.pending = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(f());
            ctx.request_repaint();
        });
    }
    pub fn load(&mut self, ctx: &Wake) {
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
    pub fn refresh_config(&mut self, ctx: &Wake) {
        self.job(ctx, || {
            let file = FileDoc::read(codex_home()?.join("config.toml"))?;
            let parsed = file
                .text
                .parse::<DocumentMut>()
                .map_err(|e| e.to_string())?;
            Ok(Job::ConfigLoaded(file, parsed))
        });
    }
}

pub fn update_mcp(
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
