use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use chrono::Utc;

use crate::filesystem::open_canonical_file;

use crate::course::model::{CourseSourceChangedEvent, MAX_MARKDOWN_BYTES};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileFingerprint {
    exists: bool,
    len: u64,
    modified: Option<SystemTime>,
    content_hash: u64,
}

pub(crate) struct CourseWatcher {
    stop_signal: Arc<(Mutex<bool>, Condvar)>,
    handle: Option<JoinHandle<()>>,
}

impl CourseWatcher {
    pub(crate) fn start<F>(course_id: String, watched_files: Vec<PathBuf>, on_change: F) -> Self
    where
        F: Fn(CourseSourceChangedEvent) + Send + 'static,
    {
        let stop_signal = Arc::new((Mutex::new(false), Condvar::new()));
        let signal_clone = Arc::clone(&stop_signal);

        let initial: Vec<(PathBuf, FileFingerprint)> = watched_files
            .into_iter()
            .map(|path| {
                let fp = compute_fingerprint(&path);
                (path, fp)
            })
            .collect();

        let handle = thread::Builder::new()
            .name("codelab-course-watcher".into())
            .spawn(move || {
                let mut baseline = initial;
                let (lock, cvar) = &*signal_clone;
                let mut stopped = lock.lock().unwrap_or_else(|poison| poison.into_inner());

                while !*stopped {
                    let (guard, _) = cvar
                        .wait_timeout(stopped, DEFAULT_POLL_INTERVAL)
                        .unwrap_or_else(|poison| poison.into_inner());
                    stopped = guard;
                    if *stopped {
                        break;
                    }
                    drop(stopped);

                    let mut changed_paths = Vec::new();
                    for (path, last_fp) in &mut baseline {
                        let current_fp = compute_fingerprint(path);
                        if current_fp != *last_fp {
                            *last_fp = current_fp;
                            changed_paths.push(path.to_string_lossy().into_owned());
                        }
                    }

                    if !changed_paths.is_empty() {
                        on_change(CourseSourceChangedEvent {
                            course_id: course_id.clone(),
                            changed_paths,
                            detected_at: Utc::now().to_rfc3339(),
                        });
                    }

                    stopped = lock.lock().unwrap_or_else(|poison| poison.into_inner());
                }
            })
            .ok();

        Self {
            stop_signal,
            handle,
        }
    }

    fn stop_inner(&mut self) {
        let (lock, cvar) = &*self.stop_signal;
        if let Ok(mut stopped) = lock.lock() {
            *stopped = true;
            cvar.notify_all();
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for CourseWatcher {
    fn drop(&mut self) {
        self.stop_inner();
    }
}

fn compute_fingerprint(path: &Path) -> FileFingerprint {
    open_canonical_file(path)
        .and_then(|file| {
            let metadata = file.metadata()?;
            let len = metadata.len();
            let content_hash = if len > MAX_MARKDOWN_BYTES as u64 {
                u64::MAX
            } else {
                let mut bytes = Vec::new();
                file.take((MAX_MARKDOWN_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)?;
                fnv1a_64(&bytes)
            };
            Ok(FileFingerprint {
                exists: true,
                len,
                modified: metadata.modified().ok().map(|time| time.into_std()),
                content_hash,
            })
        })
        .unwrap_or(FileFingerprint {
            exists: false,
            len: 0,
            modified: None,
            content_hash: 0,
        })
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}
