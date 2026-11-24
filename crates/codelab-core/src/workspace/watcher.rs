use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use notify::event::{AccessKind, AccessMode};
use notify::{Config, Event, EventKindMask, RecommendedWatcher, RecursiveMode, Watcher};

use super::paths::{is_ignored, wire_path};

const DEBOUNCE: Duration = Duration::from_millis(120);

enum Message {
    Paths(Vec<String>),
    Stop,
}

pub(super) struct WorkspaceWatcher {
    sender: mpsc::Sender<Message>,
    worker: Option<JoinHandle<()>>,
}

impl WorkspaceWatcher {
    pub fn start(
        root: PathBuf,
        on_change: impl Fn(Vec<String>) + Send + 'static,
    ) -> Result<Self, notify::Error> {
        let (sender, receiver) = mpsc::channel();
        let callback = sender.clone();
        let watch_root = root.clone();
        let watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                let paths = match result {
                    Ok(event)
                        if event.kind.is_access()
                            && !matches!(
                                event.kind,
                                notify::EventKind::Access(AccessKind::Close(AccessMode::Write))
                            ) =>
                    {
                        return;
                    }
                    Ok(event) if !event.need_rescan() => event
                        .paths
                        .iter()
                        .filter_map(|path| path.strip_prefix(&watch_root).ok())
                        .filter(|path| !is_ignored(path))
                        .filter_map(|path| wire_path(path).ok())
                        .collect::<Vec<_>>(),
                    _ => vec![String::new()],
                };
                if !paths.is_empty() {
                    let _ = callback.send(Message::Paths(paths));
                }
            },
            Config::default()
                .with_follow_symlinks(false)
                .with_event_kinds(EventKindMask::CORE | EventKindMask::ACCESS_CLOSE)
                .with_fsevent_latency(Duration::from_millis(40)),
        )?;
        let mut watches = DirectoryWatches::start(watcher, root)?;
        let worker = thread::Builder::new()
            .name("codelab-workspace-watcher".into())
            .spawn(move || {
                while let Ok(Message::Paths(first)) = receiver.recv() {
                    let mut batch = BTreeSet::from_iter(first);
                    // Bound latency even when an external build keeps writing.
                    let deadline = Instant::now() + DEBOUNCE;
                    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
                        match receiver.recv_timeout(remaining) {
                            Ok(Message::Paths(paths)) => batch.extend(paths),
                            Err(mpsc::RecvTimeoutError::Timeout) => break,
                            _ => return,
                        }
                    }
                    let changed: Vec<_> = batch.into_iter().collect();
                    watches.update(&changed);
                    on_change(changed);
                }
            })
            .map_err(notify::Error::io)?;

        Ok(Self {
            sender,
            worker: Some(worker),
        })
    }
}

struct DirectoryWatches {
    watcher: RecommendedWatcher,
    root: PathBuf,
    watched: BTreeSet<PathBuf>,
}

impl DirectoryWatches {
    fn start(watcher: RecommendedWatcher, root: PathBuf) -> Result<Self, notify::Error> {
        let mut watches = Self {
            watcher,
            root,
            watched: BTreeSet::new(),
        };
        let root = watches.root.clone();
        if cfg!(target_os = "linux") {
            // Inotify implements recursion by taking a watch for every directory.
            // Skip ignored subtrees before registration to keep those watches available.
            watches.watch_tree(&root)?;
        } else {
            watches.watcher.watch(&root, RecursiveMode::Recursive)?;
        }
        Ok(watches)
    }

    fn watch_tree(&mut self, path: &Path) -> Result<(), notify::Error> {
        if !self.watched.contains(path) {
            self.watcher.watch(path, RecursiveMode::NonRecursive)?;
            self.watched.insert(path.to_owned());
        }
        let dir = crate::filesystem::open_canonical_dir(path).map_err(notify::Error::io)?;
        for entry in dir.entries().map_err(notify::Error::io)? {
            let entry = entry.map_err(notify::Error::io)?;
            if entry.file_type().map_err(notify::Error::io)?.is_dir() {
                let child = path.join(entry.file_name());
                if child
                    .strip_prefix(&self.root)
                    .is_ok_and(|path| !is_ignored(path))
                {
                    self.watch_tree(&child)?;
                }
            }
        }
        Ok(())
    }

    fn update(&mut self, paths: &[String]) {
        if !cfg!(target_os = "linux") {
            return;
        }
        let changed: Vec<_> = paths.iter().map(|path| self.root.join(path)).collect();
        let removed: Vec<_> = self
            .watched
            .iter()
            .filter(|path| {
                changed.iter().any(|changed| path.starts_with(changed)) && !is_directory(path)
            })
            .cloned()
            .collect();
        for path in removed {
            self.watched.remove(&path);
            let _ = self.watcher.unwatch(&path);
        }
        for path in changed {
            if is_directory(&path) && (path == self.root || !self.watched.contains(&path)) {
                let _ = self.watch_tree(&path);
            }
        }
    }
}

fn is_directory(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

impl Drop for WorkspaceWatcher {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
