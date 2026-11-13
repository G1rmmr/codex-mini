use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::PathBuf};

pub const MAX_BODY: usize = 64 * 1024;
const MAX_TRANSCRIPT: usize = 2 * 1024 * 1024;
const MAX_MESSAGES: usize = 160;

pub fn clipped(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n… [화면 표시 생략]", &text[..end])
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tool: bool,
}

#[derive(Default)]
pub struct Transcript {
    pub messages: VecDeque<Message>,
    pub trimmed: bool,
}

impl Transcript {
    pub fn put(&mut self, id: &str, title: &str, body: &str, tool: bool, append: bool) {
        if let Some(message) = self.messages.iter_mut().find(|m| m.id == id) {
            if append {
                if message.body.len() < MAX_BODY {
                    let left = MAX_BODY - message.body.len();
                    message.body.push_str(&clipped(body, left));
                }
            } else {
                message.body = clipped(body, MAX_BODY);
            }
            message.title = title.to_owned();
        } else {
            self.messages.push_back(Message {
                id: id.to_owned(),
                title: title.to_owned(),
                body: clipped(body, MAX_BODY),
                tool,
            });
        }
        while self.messages.len() > MAX_MESSAGES
            || self.messages.iter().map(|m| m.body.len()).sum::<usize>() > MAX_TRANSCRIPT
        {
            self.messages.pop_front();
            self.trimmed = true;
        }
    }

    pub fn item(&mut self, item: &Value) {
        let id = string(item, "id");
        let kind = string(item, "type");
        let status = string(item, "status");
        match kind {
            "agentMessage" => self.put(id, "CODEX", string(item, "text"), false, false),
            "userMessage" => {
                let body = item["content"]
                    .as_array()
                    .map(|xs| {
                        xs.iter()
                            .filter_map(|v| v["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                self.put(id, "나", &body, false, false);
            }
            "commandExecution" => self.put(
                id,
                &format!("명령 · {status}"),
                &format!(
                    "{}\n\n{}",
                    string(item, "command"),
                    string(item, "aggregatedOutput")
                ),
                true,
                false,
            ),
            "fileChange" => {
                let mut body = String::new();
                if let Some(changes) = item["changes"].as_array() {
                    for change in changes {
                        if body.len() > MAX_BODY {
                            break;
                        }
                        body.push_str(&format!(
                            "{}\n{}\n\n",
                            string(change, "path"),
                            clipped(string(change, "diff"), MAX_BODY)
                        ));
                    }
                }
                self.put(id, &format!("파일 변경 · {status}"), &body, true, false);
            }
            "mcpToolCall" => self.put(
                id,
                &format!("도구 · {status}"),
                &clipped(
                    &format!(
                        "{} / {}\n{}",
                        string(item, "server"),
                        string(item, "tool"),
                        item["result"]
                    ),
                    MAX_BODY,
                ),
                true,
                false,
            ),
            "webSearch" => self.put(id, "웹 검색", string(item, "query"), true, false),
            "plan" => self.put(id, "계획", string(item, "text"), false, false),
            "reasoning" => {}
            _ => {}
        }
    }
}

pub fn string<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Recent {
    pub id: String,
    pub title: String,
    pub cwd: String,
    #[serde(default)]
    pub project_id: Option<u64>,
    #[serde(default)]
    pub efficiency: crate::efficiency::Options,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub cwd: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub efficiency: crate::efficiency::Options,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub cwd: String,
    pub model: String,
    pub effort: String,
    pub recent: Vec<Recent>,
    pub projects: Vec<Project>,
    pub active_project: Option<u64>,
    pub next_project_id: u64,
    pub codex_binary: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .display()
                .to_string(),
            model: String::new(),
            effort: String::new(),
            recent: Vec::new(),
            projects: Vec::new(),
            active_project: None,
            next_project_id: 1,
            codex_binary: String::new(),
        }
    }
}

impl Settings {
    pub fn active(&self) -> Option<&Project> {
        self.projects
            .iter()
            .find(|p| Some(p.id) == self.active_project)
    }
    pub fn add_project(
        &mut self,
        name: &str,
        cwd: &str,
        instructions: &str,
    ) -> Result<u64, String> {
        if self.projects.len() >= 32 {
            return Err("프로젝트는 최대 32개까지 저장할 수 있습니다.".into());
        }
        if name.trim().is_empty() || name.len() > 120 {
            return Err("프로젝트 이름은 1~120바이트로 입력하세요.".into());
        }
        if cwd.len() > 4096 || instructions.len() > 16 * 1024 {
            return Err("경로는 4 KiB, 지침은 16 KiB 이하로 입력하세요.".into());
        }
        let id = self
            .next_project_id
            .max(self.projects.iter().map(|p| p.id).max().unwrap_or(0) + 1);
        self.next_project_id = id + 1;
        self.projects.push(Project {
            id,
            name: name.trim().into(),
            cwd: cwd.into(),
            instructions: instructions.into(),
            efficiency: Default::default(),
        });
        Ok(id)
    }
    pub fn prepare_projects(&mut self) {
        // Upgrade old settings without losing their recent thread IDs.
        let mut folders: Vec<String> = self
            .recent
            .iter()
            .filter(|r| r.project_id.is_none())
            .map(|r| r.cwd.clone())
            .collect();
        folders.push(self.cwd.clone());
        for cwd in folders {
            if !self.projects.iter().any(|p| p.cwd == cwd) {
                let name = std::path::Path::new(&cwd)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("프로젝트");
                let _ = self.add_project(name, &cwd, "");
            }
        }
        for recent in &mut self.recent {
            if recent.project_id.is_none() {
                recent.project_id = self
                    .projects
                    .iter()
                    .find(|p| p.cwd == recent.cwd)
                    .map(|p| p.id);
            }
        }
        if !self.active().is_some_and(|p| p.cwd == self.cwd) {
            self.active_project = self
                .projects
                .iter()
                .find(|p| p.cwd == self.cwd)
                .map(|p| p.id);
        }
    }

    pub fn path() -> Option<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
            .map(|p| p.join("codex-mini/settings.json"))
    }
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read(p).ok())
            .filter(|b| b.len() < 4 * 1024 * 1024)
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }
    #[cfg(not(test))]
    pub fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or("설정 저장 경로가 없습니다.")?;
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(tmp, path).map_err(|e| e.to_string())
    }
    pub fn remember(&mut self, id: &str, title: &str) {
        self.recent.retain(|r| r.id != id);
        self.recent.insert(
            0,
            Recent {
                id: id.to_owned(),
                title: clipped(title, 100),
                cwd: self.cwd.clone(),
                project_id: self.active_project,
                efficiency: self
                    .active()
                    .map(|p| p.efficiency.clone())
                    .unwrap_or_default(),
            },
        );
        let active = self.active_project;
        let mut count = 0;
        self.recent.retain(|r| {
            if r.project_id == active {
                count += 1;
                count <= 20
            } else {
                true
            }
        });
    }
}

pub fn turn_params(thread: &str, prompt: &str, model: &str, effort: &str) -> Value {
    let mut p =
        json!({"threadId":thread,"input":[{"type":"text","text":prompt,"text_elements":[]}]});
    if !model.is_empty() {
        p["model"] = model.into();
    }
    if !effort.is_empty() {
        p["effort"] = effort.into();
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projects_migrate_and_keep_separate_recent_histories() {
        let mut settings: Settings = serde_json::from_value(
            json!({"cwd":"/tmp/a","recent":[{"id":"old","title":"기존 대화","cwd":"/tmp/a"}]}),
        )
        .unwrap();
        settings.prepare_projects();
        let a = settings.active_project.unwrap();
        assert_eq!(settings.recent[0].project_id, Some(a));
        let b = settings
            .add_project("두 번째", "/tmp/b", "한글 지침")
            .unwrap();
        for id in [a, b] {
            settings.active_project = Some(id);
            for n in 0..25 {
                settings.remember(&format!("{id}-{n}"), "대화");
            }
        }
        assert_eq!(
            settings
                .recent
                .iter()
                .filter(|r| r.project_id == Some(a))
                .count(),
            20
        );
        assert_eq!(
            settings
                .recent
                .iter()
                .filter(|r| r.project_id == Some(b))
                .count(),
            20
        );
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.active().unwrap().instructions, "한글 지침");
        assert_eq!(restored.recent.len(), 40);
    }
    #[test]
    fn streams_reconcile_with_final_item_without_duplicate_text() {
        let mut t = Transcript::default();
        t.put("a", "CODEX", "안", false, true);
        t.put("a", "CODEX", "녕하세요", false, true);
        t.item(&json!({"type":"agentMessage","id":"a","text":"안녕하세요!"}));
        assert_eq!(t.messages.len(), 1);
        assert_eq!(t.messages[0].body, "안녕하세요!");
    }
    #[test]
    fn transcript_is_bounded_and_preserves_utf8() {
        let mut t = Transcript::default();
        for n in 0..250 {
            t.put(&n.to_string(), "CODEX", &"한".repeat(30000), false, false);
        }
        assert!(t.trimmed);
        assert!(t.messages.len() <= MAX_MESSAGES);
        assert!(t.messages.iter().map(|m| m.body.len()).sum::<usize>() <= MAX_TRANSCRIPT);
        assert!(clipped("한글", 4).starts_with('한'));
    }
    #[test]
    fn prompts_are_json_data_and_defaults_are_inherited() {
        let prompt = "한글\n\"quoted\" $(touch /tmp/no)";
        let p = turn_params("thread", prompt, "", "");
        assert_eq!(p["input"][0]["text"], prompt);
        assert!(p.get("model").is_none());
        assert!(p.get("effort").is_none());
    }
}
