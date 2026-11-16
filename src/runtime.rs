//! Bounded, coalesced wake-ups. No interval wakes the app while it is idle.
use iced::futures::{StreamExt, channel::mpsc};
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Async UI tasks only need one worker; Codex I/O and resource sampling have
/// their own blocking workers. Avoid a Tokio thread for every logical CPU.
pub struct Executor(tokio::runtime::Runtime);
impl iced::Executor for Executor {
    fn new() -> std::io::Result<Self> {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .max_blocking_threads(2)
            .thread_name("mini-async")
            .enable_all()
            .build()
            .map(Self)
    }
    fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) {
        drop(self.0.spawn(future));
    }
    fn block_on<T>(&self, future: impl Future<Output = T>) -> T {
        self.0.block_on(future)
    }
    fn enter<R>(&self, f: impl FnOnce() -> R) -> R {
        let _guard = self.0.enter();
        f()
    }
}

#[derive(Clone)]
pub struct Wake(Arc<Inner>);
struct Inner {
    sender: Mutex<mpsc::Sender<()>>,
    receiver: Mutex<Option<mpsc::Receiver<()>>>,
}
impl Default for Wake {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel(1);
        Self(Arc::new(Inner {
            sender: Mutex::new(sender),
            receiver: Mutex::new(Some(receiver)),
        }))
    }
}
impl Hash for Wake {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}
impl Wake {
    pub fn request_repaint(&self) {
        if let Ok(mut sender) = self.0.sender.lock() {
            let _ = sender.try_send(());
        }
    }
    pub fn request_repaint_after(&self, _: Duration) {
        self.request_repaint();
    }
    pub fn subscription(&self) -> iced::Subscription<()> {
        iced::Subscription::run_with(self.clone(), |wake| {
            let receiver = wake.0.receiver.lock().expect("wake receiver lock").take();
            iced::futures::stream::unfold(receiver, |receiver| async move {
                let mut receiver = receiver?;
                receiver.next().await?;
                tokio::time::sleep(Duration::from_millis(33)).await;
                while receiver.try_recv().is_ok() {}
                Some(((), Some(receiver)))
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wake_queue_stays_bounded_under_a_stream_burst() {
        let wake = Wake::default();
        for _ in 0..10_000 {
            wake.request_repaint();
        }
        let mut receiver = wake.0.receiver.lock().unwrap().take().unwrap();
        let mut count = 0;
        while receiver.try_recv().is_ok() {
            count += 1;
        }
        assert!((1..=2).contains(&count));
        wake.request_repaint();
        assert!(receiver.try_recv().is_ok());
    }
    #[test]
    fn single_worker_executor_runs_timer_driven_tasks() {
        use iced::Executor as _;
        let executor = Executor::new().unwrap();
        let value = executor.block_on(async {
            tokio::time::sleep(Duration::from_millis(1)).await;
            42
        });
        assert_eq!(value, 42);
    }
}
