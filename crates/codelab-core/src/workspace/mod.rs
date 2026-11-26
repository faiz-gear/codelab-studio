pub(crate) mod command_cwd;
mod comparison;
mod files;
mod metadata;
mod model;
mod paths;
pub(crate) mod progress;
mod reader;
mod starter;
mod watcher;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub use comparison::WorkspaceComparison;
pub use model::{
    ExternalIde, MAX_WORKSPACE_TEXT_BYTES, WorkspaceEntry, WorkspaceEntryKind, WorkspaceFile,
    WorkspaceFileContents, WorkspaceFileRename, WorkspaceFileWrite, WorkspaceFilesChanged,
    WorkspaceIdeRequest, WorkspacePath, WorkspaceSummary,
};

use chrono::{DateTime, Utc};

use self::metadata::WorkspaceMetadata;
pub(crate) use self::paths::{WorkspacePaths, is_sensitive_path};
use self::watcher::WorkspaceWatcher;
use crate::course::{CourseSource, LoadedCourse, Starter};
use crate::storage::JsonStore;
use crate::tutor::{TutorHistory, TutorWorkspaceAttachment};
use crate::{DomainError, ErrorCode};

pub(crate) struct Workspace {
    pub summary: WorkspaceSummary,
    completed_exercises: u32,
    progress: progress::Progress,
    progress_course: progress::ProgressCourse,
    progress_store: JsonStore<progress::Progress>,
    tutor_history: Option<(JsonStore<TutorHistory>, TutorHistory)>,
    last_saved_at: Arc<Mutex<DateTime<Utc>>>,
    pub files: Arc<WorkspacePaths>,
    pub command_cwds: command_cwd::CommandCwds,
    pub git_starter_snapshot: Option<String>,
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
        // Finish metadata initialization and Starter instantiation before starting
        // the filesystem watcher so freshly bound Workspaces do not emit spurious
        // external-change events for their own Starter files.
        let metadata = WorkspaceMetadata::bind(&files, course, starter)?;

        let events_enabled = Arc::new(AtomicBool::new(true));
        let enabled = Arc::clone(&events_enabled);
        let last_saved_at = Arc::new(Mutex::new(Utc::now()));
        let saved_at = Arc::clone(&last_saved_at);
        let watched_root = root.clone();
        let watcher = WorkspaceWatcher::start(root, move |paths| {
            let latest = paths
                .iter()
                .filter(|path| {
                    Path::new(path)
                        .extension()
                        .and_then(|s| s.to_str())
                        .is_some_and(|ext| {
                            matches!(
                                ext,
                                "go" | "js"
                                    | "ts"
                                    | "tsx"
                                    | "jsx"
                                    | "py"
                                    | "java"
                                    | "json"
                                    | "yaml"
                                    | "yml"
                                    | "toml"
                                    | "mod"
                                    | "sum"
                            )
                        })
                })
                .map(|path| {
                    std::fs::metadata(watched_root.join(path))
                        .and_then(|metadata| metadata.modified())
                        .map(DateTime::<Utc>::from)
                        .unwrap_or_else(|_| Utc::now())
                })
                .max();
            if let Some(latest) = latest {
                record_change(&saved_at, latest);
            }
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

        Ok(Self {
            summary: WorkspaceSummary {
                id,
                root: root_label,
                recovery_notices: metadata.recovery_notices,
            },
            completed_exercises: metadata.completed_exercises,
            progress: metadata.progress,
            progress_course: metadata.progress_course,
            progress_store: metadata.progress_store,
            tutor_history: Some((metadata.tutor_history_store, metadata.tutor_history)),
            last_saved_at,
            files: Arc::new(files),
            command_cwds: metadata.command_cwds,
            git_starter_snapshot: metadata.git_starter_snapshot,
            events_enabled,
            _watcher: watcher,
        })
    }

    pub fn last_saved_at(&self) -> DateTime<Utc> {
        *self.last_saved_at.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub(crate) fn take_tutor_attachment(&mut self) -> Option<TutorWorkspaceAttachment> {
        let (history_store, history) = self.tutor_history.take()?;
        Some(TutorWorkspaceAttachment {
            workspace_id: self.summary.id,
            root: self.files.root.clone(),
            files: Arc::clone(&self.files),
            last_saved_at: Arc::clone(&self.last_saved_at),
            history_store,
            history,
        })
    }

    pub fn write_file(
        &self,
        path: &str,
        text: &str,
        expected: Option<&str>,
        comparison: bool,
    ) -> Result<Result<WorkspaceFile, WorkspaceFile>, DomainError> {
        let result = self.files.write(path, text, expected, comparison)?;
        if result.is_ok() {
            record_change(&self.last_saved_at, Utc::now());
        }
        Ok(result)
    }

    pub fn create_file(&self, path: &str) -> Result<WorkspaceFile, DomainError> {
        let file = self.files.create(path)?;
        record_change(&self.last_saved_at, Utc::now());
        Ok(file)
    }

    pub fn rename_file(&self, path: &str, new_path: &str) -> Result<WorkspaceFile, DomainError> {
        let file = self.files.rename(path, new_path)?;
        if path != file.path {
            record_change(&self.last_saved_at, Utc::now());
        }
        Ok(file)
    }

    pub fn completed_exercises(&self) -> u32 {
        self.completed_exercises
    }

    pub fn progress_chapters(&self) -> BTreeMap<String, crate::ChapterProgress> {
        self.progress.chapters.clone()
    }

    fn update_progress(
        &mut self,
        update: impl FnOnce(&mut progress::Progress) -> Result<(), DomainError>,
    ) -> Result<(), DomainError> {
        self.update_progress_for_course(self.progress_course.clone(), update)
    }

    fn update_progress_for_course(
        &mut self,
        course: progress::ProgressCourse,
        update: impl FnOnce(&mut progress::Progress) -> Result<(), DomainError>,
    ) -> Result<(), DomainError> {
        let mut next = self.progress.clone();
        update(&mut next)?;
        self.progress_store.save_atomic(&next)?;
        self.completed_exercises = next.completed_for(&course);
        self.progress = next;
        self.progress_course = course;
        Ok(())
    }

    pub fn start_checks<'a>(
        &mut self,
        chapter: &crate::Chapter,
        check_ids: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), DomainError> {
        self.update_progress(|progress| {
            progress.start_checks(chapter, check_ids);
            Ok(())
        })
    }

    pub fn record_check_result(
        &mut self,
        chapter_id: &str,
        check_id: &str,
        result: crate::CheckResult,
    ) -> Result<(), DomainError> {
        self.update_progress(|progress| {
            progress.record_check_result(chapter_id, check_id, result);
            Ok(())
        })
    }

    pub fn finish_checks(&mut self, chapter: &crate::Chapter) -> Result<(), DomainError> {
        self.update_progress(|progress| {
            progress.finish_checks(chapter);
            Ok(())
        })
    }

    pub fn set_self_check(
        &mut self,
        chapter: &crate::Chapter,
        check_id: &str,
        checked: bool,
    ) -> Result<(), DomainError> {
        self.update_progress(|progress| progress.set_self_check(chapter, check_id, checked))
    }

    pub fn complete_exercise(&mut self, chapter: &crate::Chapter) -> Result<(), DomainError> {
        self.update_progress(|progress| progress.complete_exercise(chapter))
    }

    pub fn reset_progress(&mut self) -> Result<(), DomainError> {
        self.update_progress(|progress| {
            progress.reset();
            Ok(())
        })
    }

    pub fn record_unsaved_check_result(
        &mut self,
        chapter_id: &str,
        check_id: &str,
        result: crate::CheckResult,
    ) {
        self.progress
            .record_check_result(chapter_id, check_id, result);
        self.completed_exercises = self.progress.completed_for(&self.progress_course);
    }

    pub fn prepare_progress_for_course(
        &mut self,
        course: &LoadedCourse,
    ) -> Result<bool, DomainError> {
        let next_course = progress::ProgressCourse::from_course(course);
        if !self.progress.has_interrupted_checks() {
            self.completed_exercises = self.progress.completed_for(&next_course);
            self.progress_course = next_course;
            return Ok(false);
        }
        self.update_progress_for_course(next_course, |progress| {
            progress.recover_interrupted();
            Ok(())
        })?;
        Ok(true)
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

fn record_change(saved_at: &Mutex<DateTime<Utc>>, changed_at: DateTime<Utc>) {
    let mut saved_at = saved_at.lock().unwrap_or_else(|p| p.into_inner());
    *saved_at = (*saved_at).max(changed_at);
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
