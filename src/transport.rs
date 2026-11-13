use serde_json::Value;
use std::{
    io::{self, BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Duration,
};

const MAX_LINE: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub enum Event {
    Message(Value),
    Diagnostic(String),
    Closed(String),
}

/// All I/O blocks on worker threads, never on egui. Both queues are bounded.
pub struct Backend {
    child: Child,
    #[cfg(windows)]
    job: windows::ProcessJob,
    outgoing: Option<SyncSender<Value>>,
    pub incoming: Receiver<Event>,
}

impl Backend {
    pub fn start(
        binary: &str,
        cwd: &str,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let mut command = Command::new(binary);
        command
            .args(["app-server", "--listen", "stdio://"])
            .current_dir(cwd);
        Self::start_command(command, wake)
    }

    pub fn start_command(
        mut command: Command,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // A private process group lets disconnect/exit reap this backend's workers,
        // without touching a Codex process owned by the desktop app or terminal.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
        }
        #[cfg(windows)]
        let job = windows::ProcessJob::new()?;
        let mut child = command.spawn()?;
        #[cfg(windows)]
        if let Err(error) = job.assign(&child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        let mut stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let (outgoing, writes) = mpsc::sync_channel::<Value>(32);
        let (events, incoming) = mpsc::sync_channel(16);
        let wake = std::sync::Arc::new(wake);
        let tx = events.clone();
        let w = wake.clone();
        thread::spawn(move || {
            while let Ok(message) = writes.recv() {
                let result = serde_json::to_writer(&mut stdin, &message)
                    .map_err(io::Error::other)
                    .and_then(|_| stdin.write_all(b"\n"))
                    .and_then(|_| stdin.flush());
                if let Err(e) = result {
                    let _ = tx.send(Event::Closed(format!("Codex 전송 실패: {e}")));
                    w();
                    break;
                }
            }
        });
        let tx = events.clone();
        let w = wake.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_line(&mut reader, MAX_LINE) {
                    Ok(Some(line)) => match serde_json::from_str(&line) {
                        Ok(message) => {
                            if tx.send(Event::Message(message)).is_err() {
                                break;
                            }
                            w();
                        }
                        Err(e) => {
                            let _ = tx.send(Event::Closed(format!("Codex 응답 형식 오류: {e}")));
                            w();
                            break;
                        }
                    },
                    other => {
                        let reason = match other {
                            Err(e) => format!("Codex 연결 오류: {e}"),
                            _ => "Codex 연결이 종료되었습니다.".to_owned(),
                        };
                        let _ = tx.send(Event::Closed(reason));
                        w();
                        break;
                    }
                }
            }
        });
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            while let Ok(Some(line)) = read_line(&mut reader, 64 * 1024) {
                // Diagnostics may be dropped under load; protocol messages may not.
                if events.try_send(Event::Diagnostic(line)).is_ok() {
                    wake();
                }
            }
        });
        Ok(Self {
            child,
            #[cfg(windows)]
            job,
            outgoing: Some(outgoing),
            incoming,
        })
    }

    pub fn send(&self, value: Value) -> Result<(), String> {
        self.outgoing
            .as_ref()
            .ok_or("연결이 종료되었습니다.")?
            .try_send(value)
            .map_err(|e| format!("전송할 수 없습니다: {e}"))
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.outgoing.take();
        #[cfg(windows)]
        self.job.terminate();
        #[cfg(unix)]
        // SAFETY: the child was spawned into a fresh process group with this ID.
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGTERM);
        }
        for _ in 0..10 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::*;

    pub struct ProcessJob(OwnedHandle);
    impl ProcessJob {
        pub fn new() -> io::Result<Self> {
            // An unnamed, non-inheritable job owns only this backend and its descendants.
            let raw = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if raw.is_null() {
                return Err(io::Error::last_os_error());
            }
            let job = Self(unsafe { OwnedHandle::from_raw_handle(raw) });
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = unsafe {
                SetInformationJobObject(
                    job.0.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    std::mem::size_of_val(&info) as u32,
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(job)
        }
        pub fn assign(&self, child: &Child) -> io::Result<()> {
            if unsafe { AssignProcessToJobObject(self.0.as_raw_handle(), child.as_raw_handle()) }
                == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
        pub fn terminate(&self) {
            unsafe {
                TerminateJobObject(self.0.as_raw_handle(), 1);
            }
        }
    }
}

fn read_line(reader: &mut impl BufRead, limit: usize) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();
    let count = reader
        .take((limit + 1) as u64)
        .read_until(b'\n', &mut bytes)?;
    if count == 0 {
        return Ok(None);
    }
    if count > limit {
        return Err(io::Error::other("protocol line exceeds memory limit"));
    }
    String::from_utf8(bytes).map(Some).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_reader_handles_korean_eof_and_oversized_messages() {
        let mut input = io::Cursor::new("안녕\n{\"id\":1}");
        assert_eq!(read_line(&mut input, 20).unwrap().unwrap(), "안녕\n");
        assert_eq!(read_line(&mut input, 20).unwrap().unwrap(), "{\"id\":1}");
        assert!(read_line(&mut input, 20).unwrap().is_none());
        assert!(read_line(&mut io::Cursor::new("12345\n"), 4).is_err());
    }
}
