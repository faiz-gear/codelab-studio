mod files;
mod metadata;
mod model;
mod paths;
mod starter;
mod watcher;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub use model::{
    ExternalIde, MAX_WORKSPACE_TEXT_BYTES, WorkspaceEntry, WorkspaceEntryKind, WorkspaceFile,
    WorkspaceFileContents, WorkspaceFileRename, WorkspaceFileWrite, WorkspaceFilesChanged,
    WorkspaceIdeRequest, WorkspacePath, WorkspaceSummary,
};

use self::metadata::WorkspaceMetadata;
use self::paths::WorkspacePaths;
use self::watcher::WorkspaceWatcher;
use crate::course::{CourseSource, LoadedCourse, Starter};
use crate::{DomainError, ErrorCode};

pub(crate) struct Workspace {
    pub summary: WorkspaceSummary,
    pub completed_exercises: u32,
    pub files: WorkspacePaths,
    events_enabled: Arc<AtomicBool>,
    _watcher: WorkspaceWatcher,
}

impl Workspace {
    pub fn bind(
        id: u64,
        raw_path: &str,
        data_dir: &Path,
        course: &LoadedCourse,
        starter: Option<&Starter>,
        on_change: impl Fn(WorkspaceFilesChanged) + Send + 'static,
    ) -> Result<Self, DomainError> {
        let path = Path::new(raw_path);
        let invalid = || DomainError::new(ErrorCode::InvalidWorkspacePath);
        if !path.is_absolute()
            || path
                .components()
                .any(|component| component == Component::ParentDir)
        {
            return Err(invalid());
        }
        let root = fs::canonicalize(path).map_err(|_| invalid())?;
        if !root.is_dir() || overlaps(&root, data_dir) {
            return Err(invalid());
        }
        let protected = match &course.source {
            CourseSource::LocalPack { path } if overlaps(&root, Path::new(path)) => {
                return Err(invalid());
            }
            CourseSource::LocalMarkdown { path } => Path::new(path)
                .strip_prefix(&root)
                .ok()
                .map(Path::to_path_buf),
            _ => None,
        };
        let files = WorkspacePaths::open(root.clone(), protected).map_err(|_| invalid())?;
        let root_label = root.to_str().ok_or_else(invalid)?.to_owned();

        let events_enabled = Arc::new(AtomicBool::new(true));
        let enabled = Arc::clone(&events_enabled);
        let watcher = WorkspaceWatcher::start(root, move |paths| {
            if enabled.load(Ordering::Acquire) {
                on_change(WorkspaceFilesChanged {
                    workspace_id: id,
                    paths,
                });
            }
        })
        .map_err(|err| {
            DomainError::new(ErrorCode::WorkspaceWatchFailed)
                .with_detail("path", &root_label)
                .with_detail("reason", err.to_string())
        })?;
        let metadata = WorkspaceMetadata::bind(&files, course, starter)?;

        Ok(Self {
            summary: WorkspaceSummary {
                id,
                root: root_label,
                recovery_notices: metadata.recovery_notices,
            },
            completed_exercises: metadata.completed_exercises,
            files,
            events_enabled,
            _watcher: watcher,
        })
    }

    pub fn set_trusted(&self, trusted: bool) {
        self.events_enabled.store(trusted, Ordering::Release);
    }

    pub fn open_in_ide(&self, request: &WorkspaceIdeRequest) -> Result<(), DomainError> {
        let path = self.files.ide_path(&request.target.path)?;
        let (launcher, app_name) = match request.ide {
            ExternalIde::VsCode => ("code", "Visual Studio Code"),
            ExternalIde::Cursor => ("cursor", "Cursor"),
            ExternalIde::GoLand => ("goland", "GoLand"),
            ExternalIde::IntelliJIdea => ("idea", "IntelliJ IDEA"),
        };
        let mut process = if cfg!(target_os = "macos") {
            let mut command = Command::new("/usr/bin/open");
            command.args(["-a", app_name]);
            command
        } else {
            Command::new(ide_launcher(launcher))
        };
        process
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let failed =
            || DomainError::new(ErrorCode::ExternalIdeOpenFailed).with_detail("ide", app_name);
        if cfg!(target_os = "macos") {
            if !process.status().is_ok_and(|status| status.success()) {
                return Err(failed());
            }
        } else {
            let mut child = process.spawn().map_err(|_| failed())?;
            // IDEs own their lifecycle. Reap a foreground launcher without blocking IPC.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Ok(())
    }
}

fn overlaps(first: &Path, second: &Path) -> bool {
    first.starts_with(second) || second.starts_with(first)
}

fn ide_launcher(name: &str) -> PathBuf {
    if cfg!(windows) {
        let search_path = std::env::var_os("PATH").unwrap_or_default();
        let mut candidates = std::env::split_paths(&search_path)
            .filter(|entry| entry.is_absolute())
            .flat_map(|entry| {
                [".exe", ".cmd", ".bat"].map(move |ext| entry.join(format!("{name}{ext}")))
            });
        if let Some(launcher) = candidates.find(|path| path.is_file()) {
            return launcher;
        }
    }
    PathBuf::from(name)
}
