use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    sync::mpsc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub const KEEP: usize = 200;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tokens {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_write_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub total_tokens: u64,
}
impl Tokens {
    fn minus(&self, other: &Self) -> Self {
        Self {
            input_tokens: self.input_tokens.saturating_sub(other.input_tokens),
            cached_input_tokens: self
                .cached_input_tokens
                .saturating_sub(other.cached_input_tokens),
            cache_write_input_tokens: self
                .cache_write_input_tokens
                .saturating_sub(other.cache_write_input_tokens),
            output_tokens: self.output_tokens.saturating_sub(other.output_tokens),
            reasoning_output_tokens: self
                .reasoning_output_tokens
                .saturating_sub(other.reasoning_output_tokens),
            total_tokens: self.total_tokens.saturating_sub(other.total_tokens),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub thread: String,
    pub turn: String,
    pub project: String,
    pub model: String,
    pub effort: String,
    pub profile: String,
    pub started_at: u64,
    pub elapsed_secs: u64,
    pub status: String,
    pub tokens: Option<Tokens>,
    pub partial: bool,
    pub tools: u64,
}
struct Active {
    record: Record,
    baseline: Option<Tokens>,
    started: Instant,
    seen_tools: Vec<String>,
}

#[derive(Default)]
pub struct Meter {
    pub records: VecDeque<Record>,
    pub total: Option<Tokens>,
    thread: String,
    active: Option<Active>,
}
impl Meter {
    pub fn attach(&mut self, thread: &str, fresh: bool) {
        self.thread = thread.into();
        self.total = fresh.then(Tokens::default);
        self.active = None;
    }
    pub fn begin(&mut self, project: &str, model: &str, effort: &str, profile: &str) {
        self.active = Some(Active {
            record: Record {
                thread: self.thread.clone(),
                project: project.into(),
                model: model.into(),
                effort: effort.into(),
                profile: profile.into(),
                status: "inProgress".into(),
                started_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                partial: self.total.is_none(),
                ..Default::default()
            },
            baseline: self.total.clone(),
            started: Instant::now(),
            seen_tools: Vec::new(),
        });
    }
    pub fn turn(&mut self, id: &str) {
        if let Some(a) = &mut self.active {
            a.record.turn = id.into();
        }
    }
    pub fn tool(&mut self, item: &Value) {
        if !matches!(
            item["type"].as_str(),
            Some(
                "commandExecution" | "mcpToolCall" | "webSearch" | "fileChange" | "dynamicToolCall"
            )
        ) {
            return;
        }
        if let (Some(a), Some(id)) = (&mut self.active, item["id"].as_str())
            && a.record.status == "inProgress"
            && a.seen_tools.len() < 10_000
            && !a.seen_tools.iter().any(|s| s == id)
        {
            a.seen_tools.push(id.into());
            a.record.tools += 1;
        }
    }
    pub fn update(&mut self, params: &Value) -> Option<Record> {
        let thread = params["threadId"].as_str()?;
        if thread != self.thread {
            return None;
        }
        let total: Tokens = serde_json::from_value(params["tokenUsage"]["total"].clone()).ok()?;
        let last: Tokens = serde_json::from_value(params["tokenUsage"]["last"].clone()).ok()?;
        // Ignore out-of-order older notifications; duplicate totals are harmless.
        if self
            .total
            .as_ref()
            .is_some_and(|t| total.total_tokens < t.total_tokens)
        {
            return None;
        }
        self.total = Some(total.clone());
        let a = self.active.as_mut()?;
        let turn = params["turnId"].as_str()?;
        if a.record.turn.is_empty() {
            a.record.turn = turn.into();
        }
        if a.record.turn != turn {
            return None;
        }
        let baseline = a.baseline.get_or_insert_with(|| total.minus(&last));
        a.record.tokens = Some(total.minus(baseline));
        if a.record.status != "inProgress" {
            let record = a.record.clone();
            self.upsert(record.clone());
            Some(record)
        } else {
            None
        }
    }
    pub fn current(&self) -> Option<&Record> {
        self.active.as_ref().map(|a| &a.record)
    }
    pub fn finish(&mut self, status: &str) -> Option<Record> {
        let a = self.active.as_mut()?;
        if a.record.status != "inProgress" {
            return None;
        }
        a.record.status = status.into();
        a.record.elapsed_secs = a.started.elapsed().as_secs();
        if a.record.turn.is_empty() {
            return None;
        }
        let record = a.record.clone();
        self.upsert(record.clone());
        Some(record)
    }
    pub fn upsert(&mut self, record: Record) {
        self.records
            .retain(|r| r.thread != record.thread || r.turn != record.turn);
        self.records.push_front(record);
        self.records
            .make_contiguous()
            .sort_by_key(|b| std::cmp::Reverse(b.started_at));
        self.records.truncate(KEEP);
    }
    pub fn merge(&mut self, records: Vec<Record>) {
        for r in records {
            if !self
                .records
                .iter()
                .any(|x| x.thread == r.thread && x.turn == r.turn)
            {
                self.upsert(r);
            }
        }
    }
}

pub enum DiskEvent {
    Loaded(Vec<Record>),
    Error(String),
}
pub struct Journal {
    tx: Option<mpsc::SyncSender<Record>>,
    pub rx: mpsc::Receiver<DiskEvent>,
    worker: Option<std::thread::JoinHandle<()>>,
}
fn path_for(dir: &std::path::Path, record: &Record) -> Option<PathBuf> {
    let valid = |s: &str| {
        !s.is_empty()
            && s.len() <= 80
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    };
    (valid(&record.thread) && valid(&record.turn))
        .then(|| dir.join(format!("{}--{}.json", record.thread, record.turn)))
}
fn read_records(dir: &std::path::Path) -> Vec<Record> {
    let mut records = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten().take(KEEP * 4) {
            let p = entry.path();
            if p.extension().is_some_and(|x| x == "json")
                && entry.metadata().is_ok_and(|m| m.len() < 16 * 1024)
                && let Ok(bytes) = fs::read(p)
                && let Ok(r) = serde_json::from_slice::<Record>(&bytes)
            {
                records.push(r);
            }
        }
    }
    records.sort_by_key(|b| std::cmp::Reverse(b.started_at));
    records
}
impl Journal {
    pub fn start(dir: PathBuf, wake: impl Fn() + Send + 'static) -> Self {
        let (tx, writes) = mpsc::sync_channel::<Record>(32);
        let (events, rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut records = read_records(&dir);
            records.truncate(KEEP);
            let _ = events.send(DiskEvent::Loaded(records));
            wake();
            while let Ok(record) = writes.recv() {
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let path = path_for(&dir, &record).ok_or("invalid usage record id")?;
                    fs::create_dir_all(&dir)?;
                    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
                    use std::io::Write;
                    let mut options = fs::OpenOptions::new();
                    options.write(true).create(true).truncate(true);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::OpenOptionsExt;
                        options.mode(0o600);
                    }
                    let mut file = options.open(&tmp)?;
                    file.write_all(&serde_json::to_vec(&record)?)?;
                    file.sync_all()?;
                    fs::rename(tmp, path)?;
                    for old in read_records(&dir).into_iter().skip(KEEP) {
                        if let Some(p) = path_for(&dir, &old) {
                            let _ = fs::remove_file(p);
                        }
                    }
                    Ok(())
                })();
                if let Err(e) = result {
                    let _ = events.send(DiskEvent::Error(format!("사용량 저장 실패: {e}")));
                    wake();
                }
            }
        });
        Self {
            tx: Some(tx),
            rx,
            worker: Some(worker),
        }
    }
    pub fn save(&self, record: Record) -> Result<(), String> {
        self.tx
            .as_ref()
            .ok_or("사용량 저장 종료됨")?
            .try_send(record)
            .map_err(|e| format!("사용량 저장 대기열 오류: {e}"))
    }
}
impl Drop for Journal {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn number(n: u64) -> String {
    let text = n.to_string();
    let len = text.len();
    text.chars()
        .enumerate()
        .fold(String::new(), |mut s, (i, c)| {
            if i > 0 && (len - i).is_multiple_of(3) {
                s.push(',');
            }
            s.push(c);
            s
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn event(total: u64, last: u64) -> Value {
        json!({"threadId":"t","turnId":"r","tokenUsage":{"total":{"inputTokens":total,"cachedInputTokens":total/2,"outputTokens":10,"reasoningOutputTokens":4,"totalTokens":total+10},"last":{"inputTokens":last,"outputTokens":5,"totalTokens":last+5}}})
    }
    #[test]
    fn duplicate_cumulative_events_and_late_completion_do_not_double_count() {
        let mut m = Meter::default();
        m.attach("t", true);
        m.begin("p", "m", "medium", "base");
        m.turn("r");
        m.update(&event(100, 100));
        m.update(&event(100, 100));
        m.update(&event(200, 100));
        assert_eq!(
            m.finish("completed").unwrap().tokens.unwrap().input_tokens,
            200
        );
        m.update(&event(250, 50));
        assert_eq!(m.records.len(), 1);
        assert_eq!(m.records[0].tokens.as_ref().unwrap().input_tokens, 250);
        assert_eq!(m.records[0].tokens.as_ref().unwrap().total_tokens, 260);
        m.update(&event(100, 100));
        assert_eq!(m.total.unwrap().input_tokens, 250);
    }
    #[test]
    fn resume_does_not_charge_old_history_to_current_turn() {
        let mut m = Meter::default();
        m.attach("t", false);
        m.begin("p", "m", "", "base");
        m.turn("r");
        m.update(&event(10000, 100));
        m.update(&event(10200, 200));
        let r = m.finish("interrupted").unwrap();
        assert!(r.partial);
        assert_eq!(r.tokens.unwrap().input_tokens, 300);
    }
    #[test]
    fn journal_round_trip_and_path_validation() {
        let dir = std::env::temp_dir().join(format!("mini-usage-test-{}", std::process::id()));
        let r = Record {
            thread: "thread".into(),
            turn: "turn".into(),
            ..Default::default()
        };
        let journal = Journal::start(dir.clone(), || {});
        journal.save(r).unwrap();
        drop(journal);
        assert_eq!(read_records(&dir).len(), 1);
        assert!(
            path_for(
                &dir,
                &Record {
                    thread: "../escape".into(),
                    turn: "t".into(),
                    ..Default::default()
                }
            )
            .is_none()
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
