//! A single background owner coalesces geometry updates and serializes disk writes.
use super::window_preferences::WindowPreferences;
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::oneshot;

#[derive(Default)]
struct Pending {
    latest: Option<(u64, WindowPreferences, Instant)>,
    version: u64,
    completed: u64,
    flushes: Vec<(u64, oneshot::Sender<()>)>,
    stopping: bool,
}

type Shared = Arc<(Mutex<Pending>, Condvar)>;

pub struct WindowPreferencesWriter {
    shared: Shared,
}

impl Default for WindowPreferencesWriter {
    fn default() -> Self {
        Self::new(WindowPreferences::path())
    }
}

impl WindowPreferencesWriter {
    fn new(path: PathBuf) -> Self {
        let shared = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("window-preferences".into())
            .spawn(move || run(worker, path))
            .expect("start window preferences writer");
        Self { shared }
    }

    pub fn update(&self, preferences: WindowPreferences) {
        let (lock, wake) = &*self.shared;
        let mut pending = lock.lock().unwrap();
        pending.version += 1;
        pending.latest = Some((
            pending.version,
            preferences,
            Instant::now() + Duration::from_millis(350),
        ));
        wake.notify_one();
    }

    /// Request an immediate serialized write; callers may await without blocking UI.
    pub fn flush(&self) -> oneshot::Receiver<()> {
        let (sender, receiver) = oneshot::channel();
        let (lock, wake) = &*self.shared;
        let mut pending = lock.lock().unwrap();
        let version = pending.version;
        if pending.completed >= version {
            let _ = sender.send(());
        } else {
            pending.flushes.push((version, sender));
            wake.notify_one();
        }
        receiver
    }
}

impl Drop for WindowPreferencesWriter {
    fn drop(&mut self) {
        let (lock, wake) = &*self.shared;
        lock.lock().unwrap().stopping = true;
        wake.notify_one();
    }
}

fn run(shared: Shared, path: PathBuf) {
    let (lock, wake) = &*shared;
    loop {
        let mut pending = lock.lock().unwrap();
        while pending.latest.is_none() && !pending.stopping {
            pending = wake.wait(pending).unwrap();
        }
        let Some((version, preferences, due)) = pending.latest else {
            return;
        };
        if !pending.stopping && pending.flushes.is_empty() && Instant::now() < due {
            let _ = wake
                .wait_timeout(pending, due.saturating_duration_since(Instant::now()))
                .unwrap();
            continue;
        }
        pending.latest = None;
        drop(pending);
        if let Err(error) = preferences.save_at(&path) {
            tracing::warn!(%error, "could not save window preferences");
        }
        let mut pending = lock.lock().unwrap();
        pending.completed = version;
        let waiters = std::mem::take(&mut pending.flushes);
        for (requested, sender) in waiters {
            if requested <= version {
                let _ = sender.send(());
            } else {
                pending.flushes.push((requested, sender));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flush_writes_the_latest_value_without_blocking_the_caller_on_io() {
        let path = std::env::temp_dir().join(format!(
            "zbase-pref-writer-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let writer = WindowPreferencesWriter::new(path.clone());
        for width in [780., 800., 940.] {
            writer.update(WindowPreferences {
                x: 10.,
                y: 20.,
                width,
                height: 880.,
            });
        }
        writer.flush().blocking_recv().unwrap();
        let saved: WindowPreferences =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved.width, 940.);
        writer.update(WindowPreferences {
            width: 1000.,
            ..saved
        });
        writer.flush().blocking_recv().unwrap();
        let saved: WindowPreferences =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            saved.width, 1000.,
            "older writes cannot finish after the flush"
        );
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }
}
