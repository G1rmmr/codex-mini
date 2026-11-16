//! Only refreshed explicitly or while the resource panel is visible.
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Clone, Debug, Default)]
pub struct ProcessRow {
    pub pid: u32,
    pub name: String,
    pub role: &'static str,
    pub cpu: f32,
    pub memory: u64,
}
#[derive(Default)]
pub struct Resources {
    system: System,
    pub rows: Vec<ProcessRow>,
}
impl Resources {
    pub fn refresh(&mut self, backends: &[u32]) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
        let own = std::process::id();
        let mut family = vec![Pid::from_u32(own)];
        loop {
            let children: Vec<_> = self
                .system
                .processes()
                .iter()
                .filter(|(pid, p)| {
                    !family.contains(pid)
                        && p.parent().is_some_and(|parent| family.contains(&parent))
                })
                .map(|(pid, _)| *pid)
                .collect();
            if children.is_empty() {
                break;
            }
            family.extend(children);
        }
        self.rows = family
            .into_iter()
            .filter_map(|pid| {
                let p = self.system.process(pid)?;
                Some(ProcessRow {
                    pid: pid.as_u32(),
                    name: p.name().to_string_lossy().into_owned(),
                    role: if pid.as_u32() == own {
                        "GUI"
                    } else if backends.contains(&pid.as_u32()) {
                        "Codex"
                    } else {
                        "Tool"
                    },
                    cpu: p.cpu_usage(),
                    memory: p.memory(),
                })
            })
            .collect();
        self.rows.sort_by_key(|row| row.pid);
    }
}

pub struct Monitor {
    requests: std::sync::mpsc::SyncSender<Vec<u32>>,
    results: std::sync::mpsc::Receiver<Vec<ProcessRow>>,
    pub rows: Vec<ProcessRow>,
}
impl Monitor {
    pub fn new(wake: crate::runtime::Wake) -> Self {
        let (requests, rx) = std::sync::mpsc::sync_channel::<Vec<u32>>(1);
        let (tx, results) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut resources = Resources::default();
            while let Ok(backends) = rx.recv() {
                resources.refresh(&backends);
                if tx.send(std::mem::take(&mut resources.rows)).is_err() {
                    break;
                }
                wake.request_repaint();
            }
        });
        Self {
            requests,
            results,
            rows: Vec::new(),
        }
    }
    pub fn request(&self, backends: Vec<u32>) {
        let _ = self.requests.try_send(backends);
    }
    pub fn poll(&mut self) {
        while let Ok(rows) = self.results.try_recv() {
            self.rows = rows;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sample_identifies_own_gui_process() {
        let mut resources = Resources::default();
        resources.refresh(&[]);
        let gui = resources
            .rows
            .iter()
            .find(|row| row.pid == std::process::id())
            .unwrap();
        assert_eq!(gui.role, "GUI");
        assert!(gui.memory > 0);
    }
}
