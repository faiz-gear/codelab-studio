use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::course::{Chapter, CommandBlock, DaemonRequirement, LoadedCourse};
use crate::execution::preparation::{CommandInvocation, ConfirmationTarget};
use crate::execution::verification::{PreparedCheck, model::CheckResult};
use crate::execution::{CommandLaunch, Directory};
use crate::model::StoreKind;
use crate::storage::JsonStore;
use crate::workspace::{self, MetricCompletionTrigger, MetricEventDetail, Workspace};
use crate::{CheckKind, CheckStatus, DomainError, ErrorCode};

use super::super::{AppEvent, AppState, Subscribers, emit_to_subscribers};
use super::{progress_snapshot, publish_progress_snapshot};

type PreparedChecks = Vec<(String, Result<(PreparedCheck, Directory), DomainError>)>;
type PreparedDaemon = Option<(DaemonRequirement, CommandLaunch, Directory)>;

pub(super) struct VerificationPlan {
    chapter: Chapter,
    language: Option<crate::CourseLanguage>,
    checks: PreparedChecks,
    daemon: PreparedDaemon,
}

impl VerificationPlan {
    pub(super) fn prepare(
        course: &LoadedCourse,
        chapter: Chapter,
        workspace: &Workspace,
    ) -> Result<Self, DomainError> {
        let language = course.language;
        let checks = chapter
            .three_step
            .as_ref()
            .map(|step| step.step3_checks.as_slice())
            .unwrap_or_default();
        let mut prepared_checks = Vec::new();
        for check in checks
            .iter()
            .filter(|check| check.kind != CheckKind::SelfCheck)
        {
            let prepared = match PreparedCheck::prepare(check, language) {
                Ok(prepared) => {
                    let cwd = check
                        .cwd
                        .as_deref()
                        .or(chapter.default_cwd.as_deref())
                        .unwrap_or(".");
                    let directory = workspace::command_cwd::directory(&workspace.files, cwd)?;
                    Ok((prepared, directory))
                }
                Err(error) => Err(error),
            };
            prepared_checks.push((check.id.clone(), prepared));
        }

        let has_runnable_checks = prepared_checks.iter().any(|(_, check)| check.is_ok());
        let daemon = chapter
            .three_step
            .as_ref()
            .and_then(|step| step.requires_daemon.as_ref())
            .filter(|_| has_runnable_checks)
            .map(|requirement| prepare_daemon(course, &chapter, workspace, requirement))
            .transpose()?;

        Ok(Self {
            chapter,
            language,
            checks: prepared_checks,
            daemon,
        })
    }

    pub(super) fn chapter(&self) -> &Chapter {
        &self.chapter
    }

    pub(super) fn check_ids(&self) -> impl Iterator<Item = &str> {
        self.checks.iter().map(|(check_id, _)| check_id.as_str())
    }

    pub(super) fn check_count(&self) -> u16 {
        self.checks.len().min(u16::MAX as usize) as u16
    }

    pub(super) fn has_daemon(&self) -> bool {
        self.daemon.is_some()
    }

    pub(super) fn confirmation_target(&self, workspace_root: PathBuf) -> ConfirmationTarget {
        let mut commands = self
            .checks
            .iter()
            .filter_map(|(_, prepared)| {
                let (check, directory) = prepared.as_ref().ok()?;
                check.invocation(directory)
            })
            .collect::<Vec<_>>();
        if let Some((_, launch, _)) = &self.daemon {
            commands.push(CommandInvocation::from_block(launch));
        }
        ConfirmationTarget::Checks {
            chapter_id: self.chapter.chapter_id.clone(),
            workspace_root,
            commands,
        }
    }
}

fn prepare_daemon(
    course: &LoadedCourse,
    chapter: &Chapter,
    workspace: &Workspace,
    requirement: &DaemonRequirement,
) -> Result<(DaemonRequirement, CommandLaunch, Directory), DomainError> {
    let candidate = chapter
        .commands
        .iter()
        .chain(course.chapters.iter().flat_map(|chapter| &chapter.commands))
        .filter(|block| matches_requirement(block, requirement))
        .enumerate()
        .max_by_key(|(index, block)| {
            (
                requirement.service_key.as_deref().is_some_and(|key| {
                    block.label.as_deref() == Some(key)
                        || block.cwd.as_deref().is_some_and(|cwd| {
                            Path::new(cwd).file_name().is_some_and(|name| name == key)
                        })
                }),
                matches_label(block, requirement),
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
                .and_then(|path| Path::new(path).parent())
                .filter(|path| !path.as_os_str().is_empty())
                .map(|path| path.to_string_lossy().into_owned())
        });
    }
    let (launch, directory) =
        workspace
            .command_cwds
            .resolve(&workspace.files, course, &block, None)?;
    Ok((requirement.clone(), launch, directory))
}

fn matches_requirement(block: &CommandBlock, requirement: &DaemonRequirement) -> bool {
    block.mode == crate::CommandMode::Daemon
        && (block.port == Some(requirement.port) || matches_label(block, requirement))
}

fn matches_label(block: &CommandBlock, requirement: &DaemonRequirement) -> bool {
    block.command.trim() == requirement.label || block.label.as_deref() == Some(&requirement.label)
}

pub(super) struct VerificationRun {
    plan: VerificationPlan,
    report: crate::runtime::PreflightReport,
    cancelled: Arc<AtomicBool>,
}

impl VerificationRun {
    pub(super) fn new(
        plan: VerificationPlan,
        report: crate::runtime::PreflightReport,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            plan,
            report,
            cancelled,
        }
    }

    pub(super) fn spawn(
        self,
        shared: Arc<Mutex<AppState>>,
        subscribers: Arc<Subscribers>,
        recents_dir: PathBuf,
    ) -> JoinHandle<()> {
        let VerificationPlan {
            chapter,
            language,
            checks,
            daemon,
        } = self.plan;
        let report = self.report;
        let cancelled = self.cancelled;

        thread::spawn(move || {
            let daemon_result = daemon
                .map(|(requirement, launch, directory)| {
                    let port = requirement.port;
                    let startup = with_state(&shared, &cancelled, |state| {
                        let latest = state.active_workspace.as_ref().unwrap().last_saved_at();
                        state
                            .execution
                            .start_daemon(launch, directory, &report, language, port, latest)
                    })
                    .ok_or_else(|| daemon_error(port))??;
                    startup.wait_until_ready(|startup| {
                        let running = with_state(&shared, &cancelled, |state| {
                            state.execution.daemon_running(startup)
                        });
                        Some(match running {
                            None => Err(daemon_error(startup.port())),
                            Some(Err(error)) => Err(error),
                            Some(Ok(true)) => Ok(startup.port_open()),
                            Some(Ok(false)) if startup.port_open() => {
                                Err(DomainError::new(ErrorCode::DaemonPortConflict)
                                    .with_detail("port", startup.port().to_string()))
                            }
                            Some(Ok(false)) => Err(daemon_error(startup.port())),
                        })
                    })
                })
                .transpose();

            for (check_id, prepared) in checks {
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
                        ..CheckResult::running(&check_id)
                    });
                if with_state(&shared, &cancelled, |state| {
                    let workspace = state.active_workspace.as_mut().unwrap();
                    let persistence = workspace.record_check_result(
                        &chapter.chapter_id,
                        &check_id,
                        result.clone(),
                    );
                    if let Err(error) = persistence {
                        workspace.record_unsaved_check_result(
                            &chapter.chapter_id,
                            &check_id,
                            result,
                        );
                        workspace.record_unsaved_check_result(
                            &chapter.chapter_id,
                            "persistence",
                            CheckResult {
                                error: Some(error),
                                status: CheckStatus::Failed,
                                ..CheckResult::running("persistence")
                            },
                        );
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
                let total_exercises = state
                    .active_course
                    .as_ref()
                    .map_or(0, |active| active.course.total_exercises);
                let workspace = state.active_workspace.as_mut().unwrap();
                match workspace.finish_checks(&chapter) {
                    Ok(true) => {
                        let _ = workspace.record_metric(MetricEventDetail::ChapterMarkedDone {
                            trigger: MetricCompletionTrigger::Verification,
                            completed_exercises: workspace.completed_exercises(),
                            total_exercises,
                        });
                    }
                    Ok(false) => {}
                    Err(error) => {
                        workspace.record_unsaved_check_result(
                            &chapter.chapter_id,
                            "persistence",
                            CheckResult {
                                error: Some(error),
                                status: CheckStatus::Failed,
                                ..CheckResult::running("persistence")
                            },
                        );
                    }
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
                        .record_unsaved_check_result(
                            &chapter.chapter_id,
                            "recentsPersistence",
                            CheckResult {
                                error: Some(error),
                                status: CheckStatus::Failed,
                                ..CheckResult::running("recentsPersistence")
                            },
                        );
                    emit_to_subscribers(
                        &subscribers,
                        AppEvent::ProgressUpdated(progress_snapshot(state)),
                    );
                }
            });
        })
    }
}

fn daemon_error(port: u16) -> DomainError {
    DomainError::new(ErrorCode::DaemonNotReady).with_detail("port", port.to_string())
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
