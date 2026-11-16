//! Native shell services. Callbacks wake Iced; no polling or per-frame OS I/O.
use crate::i18n::Language;
use crate::runtime::Wake;
use std::sync::mpsc::{self, Receiver, SyncSender};
use tray_icon::{TrayIcon, menu::MenuItem};

#[derive(Debug)]
pub enum Action {
    Show,
    Hide,
    Quit,
    Problem(String),
}

#[derive(Clone)]
struct Events {
    tx: SyncSender<Action>,
    ctx: Wake,
}

impl Events {
    fn send(&self, action: Action) {
        let _ = self.tx.try_send(action);
        self.ctx.request_repaint();
    }
}

struct Notice {
    title: String,
    body: String,
    #[cfg(windows)]
    hwnd: usize,
}

pub struct Desktop {
    // Retain the icon and menus on the event-loop/main thread (required on macOS).
    tray: Option<TrayIcon>,
    menus: Option<(MenuItem, MenuItem, MenuItem)>,
    events: Events,
    actions: Receiver<Action>,
    notices: Option<SyncSender<Notice>>,
    pub last_error: Option<String>,
    status: (Language, String),
    #[cfg(test)]
    pub sent: Vec<(String, String)>,
    #[cfg(test)]
    pub test_tray: bool,
}

impl Desktop {
    pub fn new(ctx: &Wake, lang: Language) -> Self {
        #[cfg(not(test))]
        {
            Self::native(ctx, lang)
        }
        #[cfg(test)]
        {
            Self::empty(ctx, lang)
        }
    }

    pub(crate) fn empty(ctx: &Wake, lang: Language) -> Self {
        let (tx, actions) = mpsc::sync_channel(16);
        Self {
            tray: None,
            menus: None,
            events: Events {
                tx,
                ctx: ctx.clone(),
            },
            actions,
            notices: None,
            last_error: None,
            status: (lang, String::new()),
            #[cfg(test)]
            sent: Vec::new(),
            #[cfg(test)]
            test_tray: false,
        }
    }

    #[cfg_attr(all(test, not(target_os = "linux")), allow(dead_code))]
    pub fn native(ctx: &Wake, lang: Language) -> Self {
        use tray_icon::{
            Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
            menu::{Menu, MenuEvent, PredefinedMenuItem},
        };
        let mut shell = Self::empty(ctx, lang);
        let built = (|| -> Result<_, String> {
            let menu = Menu::new();
            let open = MenuItem::with_id("open", lang.text("열기"), true, None);
            let quit = MenuItem::with_id("quit", lang.text("앱 종료"), true, None);
            let status = MenuItem::new("Codex Mini", false, None);
            menu.append_items(&[&status, &PredefinedMenuItem::separator(), &open, &quit])
                .map_err(|e| e.to_string())?;
            let events = shell.events.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.0.as_str() {
                "open" => events.send(Action::Show),
                "quit" => events.send(Action::Quit),
                _ => {}
            }));
            let events = shell.events.clone();
            TrayIconEvent::set_event_handler(Some(move |event| {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    }
                ) {
                    events.send(Action::Show);
                }
            }));
            let icon = Icon::from_rgba(
                include_bytes!("../assets/codex-mini-64.rgba").to_vec(),
                64,
                64,
            )
            .map_err(|e| e.to_string())?;
            let builder = TrayIconBuilder::new()
                .with_id("codex-mini")
                .with_tooltip("Codex Mini")
                .with_icon(icon)
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(cfg!(target_os = "macos"));
            #[cfg(windows)]
            let builder = builder.with_guid(tray_guid());
            let tray = builder.build().map_err(|e| e.to_string())?;
            Ok((tray, open, quit, status))
        })();
        match built {
            Ok((tray, open, quit, status)) => {
                shell.tray = Some(tray);
                shell.menus = Some((open, quit, status));
            }
            Err(error) => shell.last_error = Some(error),
        }
        let (tx, rx) = mpsc::sync_channel::<Notice>(8);
        let events = shell.events.clone();
        match std::thread::Builder::new()
            .name("desktop-notifications".into())
            .spawn(move || {
                while let Ok(notice) = rx.recv() {
                    if let Err(error) = deliver(&notice) {
                        events.send(Action::Problem(error));
                    }
                }
            }) {
            Ok(_) => shell.notices = Some(tx),
            Err(error) => shell.last_error = Some(error.to_string()),
        }
        if let Some(error) = &shell.last_error {
            shell.events.send(Action::Problem(error.clone()));
        }
        shell
    }

    pub fn has_tray(&self) -> bool {
        #[cfg(test)]
        if self.test_tray {
            return true;
        }
        self.tray.is_some()
    }

    pub fn request(&self, action: Action) {
        self.events.send(action);
    }
    pub fn next_action(&self) -> Option<Action> {
        self.actions.try_recv().ok()
    }

    pub fn update(&mut self, lang: Language, status: &str) {
        if self.status.0 == lang && self.status.1 == status {
            return;
        }
        self.status = (lang, status.to_owned());
        if let Some((open, quit, item)) = &self.menus {
            open.set_text(lang.text("열기"));
            quit.set_text(lang.text("앱 종료"));
            item.set_text(lang.message(status));
        }
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(format!("Codex Mini · {}", lang.message(status))));
        }
    }

    pub fn notify(&mut self, title: &str, body: &str) {
        #[cfg(test)]
        if self.notices.is_none() {
            self.sent.push((title.to_owned(), body.to_owned()));
            return;
        }
        if let Some(tx) = &self.notices {
            let notice = Notice {
                title: title.to_owned(),
                body: body.to_owned(),
                #[cfg(windows)]
                hwnd: self
                    .tray
                    .as_ref()
                    .map(|t| t.window_handle() as usize)
                    .unwrap_or(0),
            };
            if tx.try_send(notice).is_err() {
                self.last_error = Some("Notification queue is full".into());
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn deliver(notice: &Notice) -> Result<(), String> {
    use std::{collections::HashMap, time::Duration};
    let connection = zbus::blocking::connection::Builder::session()
        .map_err(|e| e.to_string())?
        .method_timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let proxy = zbus::blocking::Proxy::new(
        &connection,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .map_err(|e| e.to_string())?;
    let escape = |text: &str| {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let _: u32 = proxy
        .call(
            "Notify",
            &(
                "Codex Mini",
                0u32,
                "dialog-information",
                &notice.title,
                escape(&notice.body),
                Vec::<String>::new(),
                HashMap::<String, zbus::zvariant::Value<'_>>::new(),
                7000i32,
            ),
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn deliver(notice: &Notice) -> Result<(), String> {
    // Use a separate OS process: never replace the Cocoa/winit app delegate or
    // run a second Cocoa loop in Iced. Text is argv data, never AppleScript code.
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let mut child = Command::new("/usr/bin/osascript")
        .args(["-e", "on run argv\ndisplay notification (item 2 of argv) with title (item 1 of argv)\nend run", "--", &notice.title, &notice.body])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("osascript: {status}"))
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Notification timed out".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(windows)]
fn tray_guid() -> u128 {
    // Distinguish simultaneously running app instances in the notification area.
    0xa274c827_70c8_41f8_9725_e8d2e6856d53 ^ u128::from(std::process::id())
}

#[cfg(windows)]
fn deliver(notice: &Notice) -> Result<(), String> {
    use windows_sys::{
        Win32::UI::Shell::{
            NIF_GUID, NIF_INFO, NIIF_INFO, NIIF_RESPECT_QUIET_TIME, NIM_MODIFY, NOTIFYICONDATAW,
            Shell_NotifyIconW,
        },
        core::GUID,
    };
    fn wide<const N: usize>(text: &str) -> [u16; N] {
        let mut result = [0; N];
        // Keep UTF-16 surrogate pairs intact at the buffer boundary.
        let mut len = 0;
        for ch in text.chars() {
            let mut encoded = [0; 2];
            let units = ch.encode_utf16(&mut encoded);
            if len + units.len() >= N {
                break;
            }
            result[len..len + units.len()].copy_from_slice(units);
            len += units.len();
        }
        result
    }
    let data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: notice.hwnd as _,
        uFlags: NIF_GUID | NIF_INFO,
        guidItem: GUID::from_u128(tray_guid()),
        dwInfoFlags: NIIF_INFO | NIIF_RESPECT_QUIET_TIME,
        szInfoTitle: wide(&notice.title),
        szInfo: wide(&notice.body),
        ..Default::default()
    };
    if unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod native_tests {
    use super::*;
    use std::{collections::HashMap, time::Duration};
    type ToolTip = (String, Vec<(i32, i32, Vec<u8>)>, String, String);
    struct Watcher(SyncSender<String>);
    #[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
    impl Watcher {
        fn register_status_notifier_item(&self, service: String) {
            let _ = self.0.try_send(service);
        }
        #[zbus(property)]
        fn is_status_notifier_host_registered(&self) -> bool {
            true
        }
        #[zbus(property)]
        fn registered_status_notifier_items(&self) -> Vec<String> {
            Vec::new()
        }
        #[zbus(property)]
        fn protocol_version(&self) -> i32 {
            0
        }
    }
    struct Notifications(SyncSender<(String, String)>);
    #[zbus::interface(name = "org.freedesktop.Notifications")]
    impl Notifications {
        #[allow(clippy::too_many_arguments)]
        fn notify(
            &self,
            app: &str,
            _replaces: u32,
            _icon: &str,
            title: &str,
            body: &str,
            _actions: Vec<String>,
            _hints: HashMap<String, zbus::zvariant::OwnedValue>,
            _timeout: i32,
        ) -> u32 {
            assert_eq!(app, "Codex Mini");
            let _ = self.0.try_send((title.into(), body.into()));
            1
        }
    }

    #[test]
    #[ignore = "requires a private dbus-run-session; creates no real desktop notifications"]
    fn native_tray_activation_and_notification() {
        assert_eq!(
            std::env::var("CODEX_MINI_PRIVATE_BUS_TEST").as_deref(),
            Ok("1")
        );
        let (registered, items) = mpsc::sync_channel(8);
        let (delivered, notifications) = mpsc::sync_channel(8);
        let service = zbus::blocking::connection::Builder::session()
            .unwrap()
            .name("org.kde.StatusNotifierWatcher")
            .unwrap()
            .name("org.freedesktop.Notifications")
            .unwrap()
            .serve_at("/StatusNotifierWatcher", Watcher(registered))
            .unwrap()
            .serve_at("/org/freedesktop/Notifications", Notifications(delivered))
            .unwrap()
            .build()
            .unwrap();
        let mut desktop = Desktop::native(&Wake::default(), Language::Korean);
        assert!(desktop.has_tray(), "{:?}", desktop.last_error);
        let name = items.recv_timeout(Duration::from_secs(5)).unwrap();
        let proxy = zbus::blocking::Proxy::new(
            &service,
            name,
            "/StatusNotifierItem",
            "org.kde.StatusNotifierItem",
        )
        .unwrap();
        let _: () = proxy.call("Activate", &(0i32, 0i32)).unwrap();
        assert!(matches!(
            desktop
                .actions
                .recv_timeout(Duration::from_secs(5))
                .unwrap(),
            Action::Show
        ));
        desktop.update(Language::English, "작업 중");
        let (_, _, _, tooltip): ToolTip = proxy.get_property("ToolTip").unwrap();
        assert!(tooltip.contains("Working"), "{tooltip}");
        desktop.notify("Codex Mini · 작업 완료", "한글 <project> & 작업 완료");
        let (title, body) = notifications.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(title, "Codex Mini · 작업 완료");
        assert_eq!(body, "한글 &lt;project&gt; &amp; 작업 완료");
        drop(proxy);
        drop(desktop);
        service
            .release_name("org.kde.StatusNotifierWatcher")
            .unwrap();
        drop(service);
        let fallback = Desktop::native(&Wake::default(), Language::Korean);
        assert!(
            !fallback.has_tray(),
            "a missing host must retain the taskbar fallback"
        );
        assert!(fallback.last_error.is_some());
    }
}
