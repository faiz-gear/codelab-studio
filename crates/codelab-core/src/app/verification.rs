use super::*;
use crate::execution::verification::{self, VerificationTask};
use crate::{Chapter, ChapterKind, CheckKind, CheckResult, CheckStatus, CompletionState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

impl App {
    pub(super) fn get_progress(&self, id: u64) -> Result<AppResponse, DomainError> {
        let state = self.lock_state();
        state.workspace(id)?;
        Ok(AppResponse::Progress(progress_snapshot(&state)))
    }
    pub(super) fn run_checks(&self, request: RunChecks) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        let id = request.target.workspace_id;
        let workspace = state.workspace(id)?;
        if state
            .verification
            .as_ref()
            .is_some_and(|task| task.chapter_id.is_some())
        {
            return Err(DomainError::new(ErrorCode::VerificationBusy));
        }
        let chapter = exercise(&state, &request.target)?.clone();
        let language = state.course()?.course.language;
        let checks = chapter
            .three_step
            .as_ref()
            .map(|s| s.step3_checks.as_slice())
            .unwrap_or_default();
        let mut plan = Vec::new();
        for check in checks.iter().filter(|c| c.kind != CheckKind::SelfCheck) {
            let prepared = match verification::PreparedCheck::prepare(check, language) {
                Ok(prepared) => {
                    let cwd = check
                        .cwd
                        .as_deref()
                        .or(chapter.default_cwd.as_deref())
                        .unwrap_or(".");
                    let directory =
                        crate::workspace::command_cwd::directory(&workspace.files, cwd)?;
                    Ok((prepared, directory))
                }
                Err(error) => Err(error),
            };
            plan.push((check.id.clone(), prepared));
        }
        let has_runnable_checks = plan.iter().any(|(_, prepared)| prepared.is_ok());
        let daemon = chapter
            .three_step
            .as_ref()
            .and_then(|s| s.requires_daemon.as_ref())
            .filter(|_| has_runnable_checks)
            .map(|requirement| {
                let course = &state.course()?.course;
                let candidate = chapter
                    .commands
                    .iter()
                    .chain(course.chapters.iter().flat_map(|c| &c.commands))
                    .filter(|block| {
                        block.mode == crate::CommandMode::Daemon
                            && (block.port == Some(requirement.port)
                                || block.command.trim() == requirement.label
                                || block.label.as_deref() == Some(&requirement.label))
                    })
                    .enumerate()
                    .max_by_key(|(index, block)| {
                        (
                            requirement.service_key.as_deref().is_some_and(|key| {
                                block.label.as_deref() == Some(key)
                                    || block.cwd.as_deref().is_some_and(|cwd| {
                                        Path::new(cwd).file_name().is_some_and(|name| name == key)
                                    })
                            }),
                            block.label.as_deref() == Some(&requirement.label)
                                || block.command.trim() == requirement.label,
                            std::cmp::Reverse(*index),
                        )
                    })
                    .map(|(_, block)| block)
                    .ok_or_else(|| {
                        DomainError::new(ErrorCode::DaemonNotReady)
                            .with_detail("port", requirement.port.to_string())
                    })?;
                let mut block = candidate.clone();
                block.port = Some(requirement.port);
                if block.cwd.is_none() {
                    block.cwd = chapter.default_cwd.clone().or_else(|| {
                        chapter
                            .entry_file
                            .as_deref()
                            .and_then(|p| Path::new(p).parent())
                            .filter(|p| !p.as_os_str().is_empty())
                            .map(|p| p.to_string_lossy().into_owned())
                    });
                }
                let (launch, directory) =
                    workspace
                        .command_cwds
                        .resolve(&workspace.files, course, &block, None)?;
                Ok::<_, DomainError>((requirement.clone(), launch, directory))
            })
            .transpose()?;
        let mut commands = plan
            .iter()
            .filter_map(|(_, prepared)| {
                let (check, directory) = prepared.as_ref().ok()?;
                Some((check.command()?.to_owned(), directory.path.clone()))
            })
            .collect::<Vec<_>>();
        if let Some((_, launch, directory)) = &daemon {
            commands.push((launch.block.command.clone(), directory.path.clone()));
        }
        let block = crate::CommandBlock {
            id: format!("verification:{}", chapter.chapter_id),
            shell: "sh".into(),
            command: commands
                .iter()
                .map(|(script, _)| script.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            cwd: Some(workspace.summary.root.clone()),
            cwd_origin: crate::ValueOrigin::Explicit,
            mode: crate::CommandMode::Oneshot,
            mode_origin: crate::ValueOrigin::Explicit,
            label: Some(chapter.title.clone()),
            port: None,
            is_secondary: false,
        };
        let launch = crate::execution::CommandLaunch {
            cwd: workspace.files.root.clone(),
            script: serde_json::to_string(&commands)
                .map_err(|_| DomainError::new(ErrorCode::CourseChecksInvalid))?,
            block,
            restart_terminal_id: None,
            cwd_key: chapter.chapter_id.clone(),
        };
        let scripts = commands
            .into_iter()
            .map(|(command, _)| command)
            .collect::<Vec<_>>();
        if let Err(confirmation) =
            state
                .execution
                .approve_checks(launch, request.confirmation, &scripts)
        {
            return Ok(AppResponse::CommandConfirmationRequired(confirmation));
        }
        let mut saved = Vec::new();
        let save_result = editors::save(state.workspace(id)?, request.drafts, &mut saved);
        let conflict = match save_result {
            Ok(conflict) => conflict,
            Err(error) => return Ok(AppResponse::CommandFailed(CommandFailed { error, saved })),
        };
        if let Some(file) = conflict {
            return Ok(AppResponse::CommandSaveConflict(CommandSaveConflict {
                file,
                saved,
            }));
        }
        let workspace = state.active_workspace.as_mut().unwrap();
        let mut next = workspace.progress.clone();
        let status = next.chapters.entry(chapter.chapter_id.clone()).or_default();
        status.last_check_results.clear();
        for (check_id, _) in &plan {
            status.last_check_results.push(running_result(check_id));
        }
        next.update_completion(&chapter, false);
        if let Err(error) = workspace.progress_store.save_atomic(&next) {
            return Ok(AppResponse::CommandFailed(CommandFailed { error, saved }));
        }
        workspace.progress = next;
        state.verification = None;
        let cancelled = Arc::new(AtomicBool::new(false));
        state.verification = Some(VerificationTask {
            chapter_id: Some(chapter.chapter_id.clone()),
            cancelled: Arc::clone(&cancelled),
            handle: None,
        });
        let initial = progress_snapshot(&state);
        let shared = Arc::clone(&self.state);
        let subscribers = Arc::clone(&self.subscribers);
        let recents_dir = self.data_dir.clone();
        let report = self.runtime.snapshot();
        let handle = thread::spawn(move || {
            let daemon_result = daemon
                .map(|(requirement, launch, directory)| {
                    let port = requirement.port;
                    let daemon = with_state(&shared, &cancelled, |state| {
                        let latest = state.active_workspace.as_ref().unwrap().last_saved_at();
                        state
                            .execution
                            .start_daemon(launch, directory, &report, language, port, latest)
                    })
                    .ok_or_else(|| {
                        DomainError::new(ErrorCode::DaemonNotReady)
                            .with_detail("port", port.to_string())
                    })??;
                    daemon.wait_until_ready(|daemon| {
                        let running = with_state(&shared, &cancelled, |state| {
                            state.execution.daemon_running(daemon)
                        });
                        Some(match running {
                            None => Err(DomainError::new(ErrorCode::DaemonNotReady)
                                .with_detail("port", daemon.port().to_string())),
                            Some(Err(error)) => Err(error),
                            Some(Ok(true)) => Ok(daemon.port_open()),
                            Some(Ok(false)) if daemon.port_open() => {
                                Err(DomainError::new(ErrorCode::DaemonPortConflict)
                                    .with_detail("port", daemon.port().to_string()))
                            }
                            Some(Ok(false)) => Err(DomainError::new(ErrorCode::DaemonNotReady)
                                .with_detail("port", daemon.port().to_string())),
                        })
                    })
                })
                .transpose();
            for (check_id, prepared) in plan {
                if cancelled.load(Ordering::Acquire) {
                    return;
                }
                let result = prepared
                    .and_then(|(check, directory)| {
                        daemon_result.clone()?;
                        check.run(directory, &report, &cancelled)
                    })
                    .unwrap_or_else(|error| CheckResult {
                        error: Some(error),
                        status: CheckStatus::Failed,
                        ..running_result(&check_id)
                    });
                if with_state(&shared, &cancelled, |state| {
                    let workspace = state.active_workspace.as_mut().unwrap();
                    let status = workspace
                        .progress
                        .chapters
                        .get_mut(&chapter.chapter_id)
                        .unwrap();
                    if let Some(previous) = status
                        .last_check_results
                        .iter_mut()
                        .find(|r| r.check_id == check_id)
                    {
                        *previous = result;
                    }
                    emit_to_subscribers(
                        &subscribers,
                        AppEvent::ProgressUpdated(progress_snapshot(state)),
                    );
                })
                .is_none()
                {
                    return;
                }
            }
            with_state(&shared, &cancelled, |state| {
                let workspace = state.active_workspace.as_mut().unwrap();
                let prior_completion =
                    workspace.progress.chapters[&chapter.chapter_id].completion_state;
                workspace.progress.update_completion(&chapter, false);
                // Persist the completion separately from the latest Check Result.
                if let Err(error) = workspace.progress_store.save_atomic(&workspace.progress) {
                    let status = workspace
                        .progress
                        .chapters
                        .get_mut(&chapter.chapter_id)
                        .unwrap();
                    status.completion_state = prior_completion;
                    status.last_check_results.push(CheckResult {
                        error: Some(error),
                        status: CheckStatus::Failed,
                        ..running_result("persistence")
                    });
                }
                if let Some(task) = &mut state.verification {
                    task.chapter_id = None;
                }
                let store = JsonStore::new(&recents_dir, "recents.json", StoreKind::Recents);
                if let Err(error) = publish_progress_snapshot(state, &store, &subscribers) {
                    state
                        .active_workspace
                        .as_mut()
                        .unwrap()
                        .progress
                        .chapters
                        .get_mut(&chapter.chapter_id)
                        .unwrap()
                        .last_check_results
                        .push(CheckResult {
                            error: Some(error),
                            status: CheckStatus::Failed,
                            ..running_result("recentsPersistence")
                        });
                    emit_to_subscribers(
                        &subscribers,
                        AppEvent::ProgressUpdated(progress_snapshot(state)),
                    );
                }
            });
        });
        state.verification.as_mut().unwrap().handle = Some(handle);
        Ok(AppResponse::VerificationStarted(VerificationStarted {
            progress: initial,
            saved,
        }))
    }
    pub(super) fn set_self_check(&self, request: SetSelfCheck) -> Result<AppResponse, DomainError> {
        self.change_progress(&request.target, |progress, chapter| {
            let check = chapter
                .three_step
                .as_ref()
                .and_then(|s| {
                    s.step3_checks
                        .iter()
                        .find(|c| c.id == request.check_id && c.kind == CheckKind::SelfCheck)
                })
                .ok_or_else(|| DomainError::new(ErrorCode::CheckNotFound))?;
            progress
                .chapters
                .entry(chapter.chapter_id.clone())
                .or_default()
                .self_checks
                .insert(check.id.clone(), request.checked);
            progress.update_completion(chapter, false);
            Ok(())
        })
    }
    pub(super) fn complete_exercise(
        &self,
        target: ExerciseTarget,
    ) -> Result<AppResponse, DomainError> {
        self.change_progress(&target, |progress, chapter| {
            if chapter.three_step.as_ref().is_some_and(|s| {
                s.step3_checks
                    .iter()
                    .any(|c| c.kind != CheckKind::SelfCheck)
            }) {
                return Err(DomainError::new(ErrorCode::ExerciseCompletionBlocked));
            }
            progress.update_completion(chapter, true);
            if progress.chapters[&chapter.chapter_id].completion_state != CompletionState::Completed
            {
                return Err(DomainError::new(ErrorCode::ExerciseCompletionBlocked));
            }
            Ok(())
        })
    }
    fn change_progress(
        &self,
        target: &ExerciseTarget,
        change: impl FnOnce(
            &mut crate::workspace::progress::Progress,
            &Chapter,
        ) -> Result<(), DomainError>,
    ) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        state.workspace(target.workspace_id)?;
        let chapter = exercise(&state, target)?.clone();
        let workspace = state.active_workspace.as_mut().unwrap();
        let mut next = workspace.progress.clone();
        change(&mut next, &chapter)?;
        workspace.progress_store.save_atomic(&next)?;
        workspace.progress = next;
        self.publish_progress(&mut state)
    }
    pub(super) fn reset_progress(&self, id: u64) -> Result<AppResponse, DomainError> {
        let mut state = self.lock_state();
        state.workspace(id)?;
        state.verification = None;
        let workspace = state.active_workspace.as_mut().unwrap();
        let next = Default::default();
        workspace.progress_store.save_atomic(&next)?;
        workspace.progress = next;
        self.publish_progress(&mut state)
    }
    fn publish_progress(&self, state: &mut AppState) -> Result<AppResponse, DomainError> {
        publish_progress_snapshot(state, &self.recents_store, &self.subscribers)
            .map(AppResponse::Progress)
    }
    pub(super) fn export_learning_record(&self, id: u64) -> Result<AppResponse, DomainError> {
        let state = self.lock_state();
        state.workspace(id)?;
        Ok(AppResponse::LearningRecord(
            serde_json::json!({ "schemaVersion": 1, "courseId": state.course()?.course.course_id, "exportedAt": Utc::now().to_rfc3339(), "progress": progress_snapshot(&state) }),
        ))
    }
}
fn exercise<'a>(state: &'a AppState, target: &ExerciseTarget) -> Result<&'a Chapter, DomainError> {
    state.workspace(target.workspace_id)?;
    state
        .course()?
        .course
        .chapters
        .iter()
        .find(|c| c.chapter_id == target.chapter_id && c.kind == ChapterKind::Exercise)
        .ok_or_else(|| DomainError::new(ErrorCode::CheckNotFound))
}
fn progress_snapshot(state: &AppState) -> LearningProgress {
    let workspace = state.active_workspace.as_ref().unwrap();
    let course = &state.active_course.as_ref().unwrap().course;
    LearningProgress {
        workspace_id: workspace.summary.id,
        chapters: workspace.progress.chapters.clone(),
        completed_exercises: workspace.progress.completed_for(course),
        total_exercises: course.total_exercises,
        running_chapter_id: state
            .verification
            .as_ref()
            .and_then(|task| task.chapter_id.clone()),
    }
}
fn running_result(id: &str) -> CheckResult {
    CheckResult {
        check_id: id.into(),
        status: CheckStatus::Running,
        duration_ms: 0,
        diffs: Vec::new(),
        raw_output: String::new(),
        test_cases: Vec::new(),
        error: None,
    }
}
fn with_state<T>(
    state: &Mutex<AppState>,
    cancelled: &AtomicBool,
    action: impl FnOnce(&mut AppState) -> T,
) -> Option<T> {
    loop {
        if cancelled.load(Ordering::Acquire) {
            return None;
        }
        if let Ok(mut state) = state.try_lock() {
            return Some(action(&mut state));
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn publish_progress_snapshot(
    state: &mut AppState,
    store: &JsonStore<Vec<RecentWorkspace>>,
    subscribers: &Subscribers,
) -> Result<LearningProgress, DomainError> {
    let course = &state.active_course.as_ref().unwrap().course;
    let workspace = state.active_workspace.as_mut().unwrap();
    workspace.completed_exercises = workspace.progress.completed_for(course);
    let mut recents = state.recents.clone();
    for recent in &mut recents {
        if recent.workspace_path == workspace.summary.root {
            recent.completed_exercises = workspace.completed_exercises;
            recent.total_exercises = course.total_exercises;
        }
    }
    store.save_atomic(&recents)?;
    state.recents = recents;
    let snapshot = progress_snapshot(state);
    emit_to_subscribers(subscribers, AppEvent::RecentsUpdated(state.recents.clone()));
    emit_to_subscribers(subscribers, AppEvent::ProgressUpdated(snapshot.clone()));
    Ok(snapshot)
}
