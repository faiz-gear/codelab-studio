use std::collections::BTreeMap;
use std::fs;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::error::{AppError, ErrorCode};
use crate::domain::events::StorageCorruptedEvent;
use crate::domain::ids::ChapterId;
use crate::domain::workspace::{
    ChapterChatLog, ChatHistory, ChatMessage, ChatRole, CourseSourceRef, LayoutSnapshot,
    ProgressState, StorageSchemaVersion, WorkspaceMeta,
};
use crate::platform::WorkspacePaths;
use crate::storage::atomic_json::{
    cleanup_stale_tmp_files, AtomicJsonFile, LoadOutcome, StorageError,
};

pub const MAX_CHAT_TURNS_PER_CHAPTER: usize = 50;
pub const MAX_CHAT_MESSAGES_PER_CHAPTER: usize = MAX_CHAT_TURNS_PER_CHAPTER * 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInitSpec {
    pub course_id: String,
    pub course_source: CourseSourceRef,
    pub pack_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceOpenReport {
    pub created_new: bool,
    pub cleaned_tmp_files: Vec<String>,
    pub recovered_from_backup: Vec<String>,
    pub corruption_events: Vec<StorageCorruptedEvent>,
}

#[derive(Debug)]
pub struct WorkspaceStore {
    paths: WorkspacePaths,
    meta_file: AtomicJsonFile<WorkspaceMeta>,
    progress_file: AtomicJsonFile<ProgressState>,
    chat_file: AtomicJsonFile<ChatHistory>,
}

impl WorkspaceStore {
    pub fn open_or_create(
        paths: WorkspacePaths,
        spec: WorkspaceInitSpec,
    ) -> Result<(Self, WorkspaceOpenReport), StorageError> {
        let meta_dir = paths.meta_dir();
        fs::create_dir_all(&meta_dir).map_err(|source| StorageError::Io {
            path: meta_dir.clone(),
            source,
        })?;

        let cleaned_tmp_files =
            cleanup_stale_tmp_files(&meta_dir).map_err(|source| StorageError::Io {
                path: meta_dir.clone(),
                source,
            })?;

        let meta_file = AtomicJsonFile::<WorkspaceMeta>::new(paths.workspace_json());
        let progress_file = AtomicJsonFile::<ProgressState>::new(paths.progress_json());
        let chat_file = AtomicJsonFile::<ChatHistory>::new(paths.chat_json());

        let mut recovered_from_backup = Vec::new();
        let mut corruption_events = Vec::new();

        let canonical_root = paths.root_string();
        let meta_outcome = meta_file.load_or_init(|| WorkspaceMeta {
            schema_version: StorageSchemaVersion::V1,
            workspace_id: format!("ws_{}", Uuid::new_v4().simple()),
            workspace_root: canonical_root.clone(),
            course_source: spec.course_source.clone(),
            pack_fingerprint: spec.pack_fingerprint.clone(),
            hydrated_files: Vec::new(),
            active_chapter_id: None,
            open_tabs: Vec::new(),
            active_tab: None,
            layout: LayoutSnapshot {
                manual_width_pct: 35,
                bottom_height_px: 256,
                focus_mode: false,
            },
            updated_at: now_rfc3339(),
        })?;

        let created_new = matches!(meta_outcome, LoadOutcome::CreatedDefault(_));
        collect_outcome_diagnostics(
            "workspace.json",
            &meta_outcome,
            &mut recovered_from_backup,
            &mut corruption_events,
        );

        let progress_outcome = progress_file.load_or_init(|| ProgressState {
            schema_version: StorageSchemaVersion::V1,
            course_id: spec.course_id.clone(),
            chapters: BTreeMap::new(),
        })?;
        collect_outcome_diagnostics(
            "progress.json",
            &progress_outcome,
            &mut recovered_from_backup,
            &mut corruption_events,
        );

        if progress_outcome.value().course_id != spec.course_id {
            return Err(StorageError::Domain(
                AppError::new(ErrorCode::SchemaValidationFailed, "error.invalidInput")
                    .with_param("field", "workspaceRoot")
                    .with_detail("this workspace belongs to another course"),
            ));
        }
        let source_changed =
            !created_new && meta_outcome.value().course_source != spec.course_source;
        let fingerprint_changed =
            !created_new && meta_outcome.value().pack_fingerprint != spec.pack_fingerprint;
        let root_changed = !created_new && meta_outcome.value().workspace_root != canonical_root;

        let chat_outcome = chat_file.load_or_init(|| ChatHistory {
            schema_version: StorageSchemaVersion::V1,
            chapters: BTreeMap::new(),
        })?;
        collect_outcome_diagnostics(
            "chat.json",
            &chat_outcome,
            &mut recovered_from_backup,
            &mut corruption_events,
        );

        if root_changed || source_changed || fingerprint_changed {
            meta_file.update(|meta| {
                meta.workspace_root = canonical_root.clone();
                meta.course_source = spec.course_source.clone();
                meta.pack_fingerprint = spec.pack_fingerprint.clone();
                meta.updated_at = now_rfc3339();
                Ok(())
            })?;
        }

        let store = Self {
            paths,
            meta_file,
            progress_file,
            chat_file,
        };
        let report = WorkspaceOpenReport {
            created_new,
            cleaned_tmp_files,
            recovered_from_backup,
            corruption_events,
        };

        Ok((store, report))
    }

    pub fn paths(&self) -> &WorkspacePaths {
        &self.paths
    }

    pub fn read_meta(&self) -> Result<WorkspaceMeta, StorageError> {
        self.meta_file.read()
    }

    pub fn update_meta<R, F>(&self, mutator: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut WorkspaceMeta) -> Result<R, StorageError>,
    {
        self.meta_file.update(|meta| {
            let result = mutator(meta)?;
            meta.updated_at = now_rfc3339();
            Ok(result)
        })
    }

    pub fn read_progress(&self) -> Result<ProgressState, StorageError> {
        self.progress_file.read()
    }

    pub fn update_progress<R, F>(&self, mutator: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut ProgressState) -> Result<R, StorageError>,
    {
        self.progress_file.update(mutator)
    }

    pub fn read_chat(&self) -> Result<ChatHistory, StorageError> {
        self.chat_file.read()
    }

    pub fn get_chapter_chat(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Option<ChapterChatLog>, StorageError> {
        let history = self.chat_file.read()?;
        Ok(history.chapters.get(chapter_id).cloned())
    }

    pub fn append_chat_message(
        &self,
        chapter_id: ChapterId,
        message: ChatMessage,
    ) -> Result<ChapterChatLog, StorageError> {
        self.chat_file.update(|history| {
            let updated_at = now_rfc3339();
            let entry = history
                .chapters
                .entry(chapter_id.clone())
                .or_insert_with(|| ChapterChatLog {
                    chapter_id: chapter_id.clone(),
                    messages: Vec::new(),
                    updated_at: updated_at.clone(),
                });
            entry.messages.push(message);
            trim_chat_messages(&mut entry.messages);
            entry.updated_at = updated_at;
            Ok(entry.clone())
        })
    }

    pub fn clear_chat(&self, chapter_id: Option<&ChapterId>) -> Result<ChatHistory, StorageError> {
        self.chat_file.update(|history| {
            match chapter_id {
                Some(id) => {
                    history.chapters.remove(id);
                }
                None => {
                    history.chapters.clear();
                }
            }
            Ok(history.clone())
        })
    }
}

fn collect_outcome_diagnostics<T>(
    file_name: &str,
    outcome: &LoadOutcome<T>,
    recovered_from_backup: &mut Vec<String>,
    corruption_events: &mut Vec<StorageCorruptedEvent>,
) {
    match outcome {
        LoadOutcome::RecoveredFromBackup { .. } => {
            recovered_from_backup.push(file_name.to_owned());
        }
        LoadOutcome::ResetAfterCorruption { event, .. } => {
            corruption_events.push(event.clone());
        }
        LoadOutcome::CreatedDefault(_) | LoadOutcome::Loaded(_) | LoadOutcome::Migrated { .. } => {}
    }
}

fn trim_chat_messages(messages: &mut Vec<ChatMessage>) {
    loop {
        let user_turns = messages
            .iter()
            .filter(|msg| msg.role == ChatRole::User)
            .count();
        if user_turns <= MAX_CHAT_TURNS_PER_CHAPTER {
            break;
        }
        let first_user = messages
            .iter()
            .position(|msg| msg.role == ChatRole::User)
            .unwrap_or(0);
        let next_user = messages
            .iter()
            .enumerate()
            .skip(first_user + 1)
            .find(|(_, msg)| msg.role == ChatRole::User)
            .map(|(idx, _)| idx)
            .unwrap_or(first_user + 1);
        messages.drain(0..next_user);
    }

    if messages.len() > MAX_CHAT_MESSAGES_PER_CHAPTER {
        let excess = messages.len() - MAX_CHAT_MESSAGES_PER_CHAPTER;
        messages.drain(0..excess);
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    use crate::domain::ai::TeachingMode;
    use crate::domain::ids::{RelWorkspacePath, RunId};
    use crate::domain::workspace::{AutoCheckRecord, ChapterProgress, ChapterStatus};

    fn sample_spec() -> WorkspaceInitSpec {
        WorkspaceInitSpec {
            course_id: "go-goframe-practice".to_owned(),
            course_source: CourseSourceRef::Builtin {
                course_id: "go-goframe-practice".to_owned(),
                entry_file: RelWorkspacePath::new("README.md").unwrap(),
            },
            pack_fingerprint: "sha256:test-fingerprint".to_owned(),
        }
    }

    #[test]
    fn creates_new_workspace_and_binds_existing_directory_without_touching_learner_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let learner_file = dir.path().join("main.go");
        fs::write(&learner_file, "package main\nfunc main() {}\n").expect("write learner file");

        let paths = WorkspacePaths::from_root(dir.path(), false).expect("paths");
        let (store, open1) =
            WorkspaceStore::open_or_create(paths.clone(), sample_spec()).expect("create store");
        assert!(open1.created_new);
        assert!(open1.corruption_events.is_empty());

        let initial_id = store.read_meta().unwrap().workspace_id;
        store
            .update_meta(|meta| {
                meta.active_chapter_id = Some(ChapterId("ex-1-healthz".to_owned()));
                meta.open_tabs = vec![RelWorkspacePath::new("main.go").unwrap()];
                meta.active_tab = Some(RelWorkspacePath::new("main.go").unwrap());
                meta.hydrated_files = vec![RelWorkspacePath::new("main.go").unwrap()];
                Ok(())
            })
            .expect("update meta");

        store
            .update_progress(|progress| {
                progress.chapters.insert(
                    ChapterId("ex-1-healthz".to_owned()),
                    ChapterProgress {
                        status: ChapterStatus::Completed,
                        completed_at: Some("2026-11-23T00:00:00Z".to_owned()),
                        last_run_id: Some(RunId("run_1".to_owned())),
                        auto_checks: BTreeMap::from([(
                            "v1".to_owned(),
                            AutoCheckRecord {
                                passed: true,
                                latency_ms: 12,
                                checked_at: "2026-11-23T00:00:00Z".to_owned(),
                            },
                        )]),
                        self_checks: BTreeMap::from([("s1".to_owned(), true)]),
                    },
                );
                Ok(())
            })
            .expect("update progress");

        drop(store);

        let (rebound, open2) =
            WorkspaceStore::open_or_create(paths, sample_spec()).expect("bind existing");
        assert!(!open2.created_new);
        assert!(open2.corruption_events.is_empty());

        let rebound_meta = rebound.read_meta().unwrap();
        assert_eq!(rebound_meta.workspace_id, initial_id);
        assert_eq!(
            rebound_meta.active_chapter_id,
            Some(ChapterId("ex-1-healthz".to_owned()))
        );
        assert_eq!(rebound_meta.layout.manual_width_pct, 35);
        assert_eq!(rebound_meta.layout.bottom_height_px, 256);

        let rebound_progress = rebound.read_progress().unwrap();
        assert_eq!(
            rebound_progress
                .chapters
                .get(&ChapterId("ex-1-healthz".to_owned()))
                .map(|c| c.status),
            Some(ChapterStatus::Completed)
        );
        assert_eq!(
            fs::read_to_string(&learner_file).expect("learner file preserved"),
            "package main\nfunc main() {}\n"
        );
    }

    #[test]
    fn binding_another_course_preserves_existing_workspace_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).expect("paths");
        let (store, _) =
            WorkspaceStore::open_or_create(paths.clone(), sample_spec()).expect("create store");
        let chapter_id = ChapterId("ex-1-healthz".to_owned());

        store
            .update_meta(|meta| {
                meta.active_chapter_id = Some(chapter_id.clone());
                meta.open_tabs = vec![RelWorkspacePath::new("main.go").unwrap()];
                meta.active_tab = Some(RelWorkspacePath::new("main.go").unwrap());
                meta.hydrated_files = vec![RelWorkspacePath::new("main.go").unwrap()];
                Ok(())
            })
            .expect("write old course state");
        store
            .update_progress(|progress| {
                progress.chapters.insert(
                    chapter_id.clone(),
                    ChapterProgress {
                        status: ChapterStatus::Completed,
                        completed_at: Some("2026-11-23T00:00:00Z".to_owned()),
                        last_run_id: None,
                        auto_checks: BTreeMap::new(),
                        self_checks: BTreeMap::new(),
                    },
                );
                Ok(())
            })
            .expect("write old course progress");
        store
            .append_chat_message(
                chapter_id.clone(),
                ChatMessage {
                    id: "old-message".to_owned(),
                    role: ChatRole::User,
                    content: "old course question".to_owned(),
                    teaching_mode: Some(TeachingMode::Direct),
                    created_at: "2026-11-23T00:00:00Z".to_owned(),
                },
            )
            .expect("write old course chat");
        let original_meta = fs::read(paths.workspace_json()).unwrap();
        let original_progress = fs::read(paths.progress_json()).unwrap();
        let original_chat = fs::read(paths.chat_json()).unwrap();
        drop(store);

        let mut next_spec = sample_spec();
        next_spec.course_id = "another-course".to_owned();
        next_spec.course_source = CourseSourceRef::GitRepo {
            url: "https://example.com/another-course.git".to_owned(),
            git_ref: Some("main".to_owned()),
            entry_file: RelWorkspacePath::new("README.md").unwrap(),
        };
        next_spec.pack_fingerprint = "sha256:another-course".to_owned();

        let error = WorkspaceStore::open_or_create(paths.clone(), next_spec)
            .expect_err("a different course must use another workspace");
        assert_eq!(
            AppError::from(error).code,
            ErrorCode::SchemaValidationFailed
        );
        assert_eq!(fs::read(paths.workspace_json()).unwrap(), original_meta);
        assert_eq!(fs::read(paths.progress_json()).unwrap(), original_progress);
        assert_eq!(fs::read(paths.chat_json()).unwrap(), original_chat);
    }

    #[test]
    fn chat_history_keeps_latest_50_turns_and_clears_atomically() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).unwrap();
        let (store, _) = WorkspaceStore::open_or_create(paths, sample_spec()).unwrap();
        let chapter = ChapterId("ex-1".to_owned());

        for turn in 0..60 {
            store
                .append_chat_message(
                    chapter.clone(),
                    ChatMessage {
                        id: format!("u_{turn}"),
                        role: ChatRole::User,
                        content: format!("question {turn}"),
                        teaching_mode: Some(TeachingMode::Socratic),
                        created_at: "2026-11-23T00:00:00Z".to_owned(),
                    },
                )
                .unwrap();
            store
                .append_chat_message(
                    chapter.clone(),
                    ChatMessage {
                        id: format!("a_{turn}"),
                        role: ChatRole::Assistant,
                        content: format!("answer {turn}"),
                        teaching_mode: Some(TeachingMode::Socratic),
                        created_at: "2026-11-23T00:00:01Z".to_owned(),
                    },
                )
                .unwrap();
        }

        let log = store.get_chapter_chat(&chapter).unwrap().unwrap();
        assert_eq!(log.messages.len(), MAX_CHAT_MESSAGES_PER_CHAPTER);
        assert_eq!(log.messages.first().unwrap().id, "u_10");
        assert_eq!(log.messages.last().unwrap().id, "a_59");

        store.clear_chat(Some(&chapter)).unwrap();
        assert!(store.get_chapter_chat(&chapter).unwrap().is_none());
    }

    #[test]
    fn concurrent_multi_file_and_same_file_writes_preserve_all_updates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::from_root(dir.path(), false).unwrap();
        let (store, _) = WorkspaceStore::open_or_create(paths, sample_spec()).unwrap();
        let store = Arc::new(store);

        let mut handles = Vec::new();
        for worker in 0..8 {
            let store = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                for step in 0..5 {
                    let chapter_id = ChapterId(format!("w{worker}_c{step}"));
                    store
                        .update_progress(|progress| {
                            progress.chapters.insert(
                                chapter_id.clone(),
                                ChapterProgress {
                                    status: ChapterStatus::Completed,
                                    completed_at: Some("2026-11-23T00:00:00Z".to_owned()),
                                    last_run_id: None,
                                    auto_checks: BTreeMap::new(),
                                    self_checks: BTreeMap::new(),
                                },
                            );
                            Ok(())
                        })
                        .expect("concurrent progress write");

                    store
                        .append_chat_message(
                            chapter_id,
                            ChatMessage {
                                id: format!("m_{worker}_{step}"),
                                role: ChatRole::User,
                                content: format!("hello {worker}-{step}"),
                                teaching_mode: Some(TeachingMode::Direct),
                                created_at: "2026-11-23T00:00:00Z".to_owned(),
                            },
                        )
                        .expect("concurrent chat write");
                }
            }));
        }

        for handle in handles {
            handle.join().expect("join writer thread");
        }

        assert_eq!(store.read_progress().unwrap().chapters.len(), 40);
        assert_eq!(store.read_chat().unwrap().chapters.len(), 40);
    }
}
