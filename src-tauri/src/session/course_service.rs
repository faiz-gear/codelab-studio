use std::path::PathBuf;
use std::sync::Arc;

use crate::course::pack_loader::{count_effective_chapters, reload_loaded_pack};
use crate::course::{
    build_inspect_result, fetch_git_pack, hydrate_starter_into_workspace, load_builtin_pack,
    load_git_pack_dir, load_local_markdown, load_local_pack_dir, validate_git_ref,
    validate_git_url, GitImportConfig, HydrationReport, LoadedCoursePack,
};
use crate::domain::course::{
    CourseInspectResult, CourseSourceInput, OpenWorkspaceReq, OpenedWorkspaceView, ParsedCourse,
};
use crate::domain::error::{AppError, ErrorCode};
use crate::domain::ids::SessionEpoch;
use crate::domain::metrics::MetricEvent;
use crate::domain::runtime::Language;
use crate::domain::workspace::CourseSourceRef;
use crate::session::app_state::{AppState, SessionLifecycleKind};
use crate::session::workspace_session::SessionOpenParams;
use crate::storage::WorkspaceInitSpec;

impl AppState {
    pub async fn inspect_course_source(
        &self,
        source: CourseSourceInput,
    ) -> Result<CourseInspectResult, AppError> {
        self.inspect_course_source_with_git_config(source, &GitImportConfig::default())
            .await
    }

    pub async fn inspect_course_source_with_git_config(
        &self,
        source: CourseSourceInput,
        git_config: &GitImportConfig,
    ) -> Result<CourseInspectResult, AppError> {
        let pack = self.load_course_source(source, git_config).await?;
        let trusted = if pack.requires_trust {
            self.global_store()
                .is_pack_trusted(&pack.source_key, &pack.pack_fingerprint)
                .map_err(AppError::from)?
        } else {
            true
        };

        if pack.requires_trust && !trusted {
            let lifecycle = self.lifecycle_state();
            if matches!(
                lifecycle,
                SessionLifecycleKind::Idle | SessionLifecycleKind::TrustPending { .. }
            ) {
                let _ = self
                    .begin_trust_pending(&pack.source_key, &pack.pack_fingerprint)
                    .await;
            }
        }

        Ok(build_inspect_result(&pack, trusted))
    }

    pub async fn trust_and_open_course(
        &self,
        req: OpenWorkspaceReq,
    ) -> Result<OpenedWorkspaceView, AppError> {
        self.trust_and_open_course_with_git_config(req, &GitImportConfig::default())
            .await
    }

    pub async fn trust_and_open_course_with_git_config(
        &self,
        req: OpenWorkspaceReq,
        git_config: &GitImportConfig,
    ) -> Result<OpenedWorkspaceView, AppError> {
        let workspace_root_trimmed = req.workspace_root.trim();
        if workspace_root_trimmed.is_empty() {
            return Err(
                AppError::new(ErrorCode::PathOutsideSandbox, "error.invalidInput")
                    .with_param("field", "workspaceRoot")
                    .with_detail("workspace root must not be empty"),
            );
        }

        let pack = self.load_course_source(req.source, git_config).await?;

        if let Some(first_issue) = pack.validation_issues.first() {
            return Err(AppError::new(
                ErrorCode::SchemaValidationFailed,
                "error.schemaValidationFailed",
            )
            .with_param("file", first_issue.file.as_str())
            .with_param("detail", first_issue.message.as_str())
            .with_detail(format!("{}: {}", first_issue.file, first_issue.message)));
        }

        if pack.requires_trust {
            let already_trusted = self
                .global_store()
                .is_pack_trusted(&pack.source_key, &pack.pack_fingerprint)
                .map_err(AppError::from)?;

            if let Some(expected) = req.expected_fingerprint.as_deref() {
                if expected != pack.pack_fingerprint {
                    return Err(AppError::new(
                        ErrorCode::ExternalPackUntrusted,
                        "error.externalPackUntrusted",
                    )
                    .with_param("source", pack.source_key.as_str())
                    .with_detail(format!(
                        "pack fingerprint mismatch: expected {expected}, actual {}",
                        pack.pack_fingerprint
                    )));
                }
            }

            if !already_trusted {
                if !req.accept_trust {
                    return Err(AppError::new(
                        ErrorCode::ExternalPackUntrusted,
                        "error.externalPackUntrusted",
                    )
                    .with_param("source", pack.source_key.as_str())
                    .with_detail("external course pack requires explicit trust confirmation"));
                }
                self.global_store()
                    .trust_pack(&pack.source_key, &pack.pack_fingerprint)
                    .map_err(AppError::from)?;
            }
        }

        let total_chapters = count_effective_chapters(&pack.course);
        let language = pack
            .manifest
            .as_ref()
            .map(|m| m.language)
            .or(pack.course.language)
            .unwrap_or(Language::Go);

        let mut params = SessionOpenParams::new(
            PathBuf::from(workspace_root_trimmed),
            req.create_if_missing,
            WorkspaceInitSpec {
                course_id: pack.course.course_id.clone(),
                course_source: pack.source.clone(),
                pack_fingerprint: pack.pack_fingerprint.clone(),
            },
            pack.course.title.clone(),
            language,
            total_chapters,
        );
        params.loaded_course = Some(pack.clone());

        let session = self.open_workspace(params).await?;

        let hydration = if let Some(starter_dir) = pack.starter_dir_abs.clone() {
            let session_for_hydrate = Arc::clone(&session);
            let starter_fingerprints = pack.starter_fingerprints.clone();
            let hydrate_res = tokio::task::spawn_blocking(move || {
                hydrate_starter_into_workspace(
                    &starter_dir,
                    session_for_hydrate.paths().root(),
                    session_for_hydrate.store(),
                    &starter_fingerprints,
                )
            })
            .await
            .map_err(join_error);

            match hydrate_res {
                Ok(Ok(report)) => report,
                Ok(Err(err)) | Err(err) => {
                    let _ = self.close_workspace(session.epoch()).await;
                    return Err(err);
                }
            }
        } else {
            HydrationReport::default()
        };

        let source_kind = match &pack.source {
            CourseSourceRef::Builtin { .. } => "builtin",
            CourseSourceRef::LocalMarkdown { .. } => "local_markdown",
            CourseSourceRef::LocalPackDir { .. } => "local_pack_dir",
            CourseSourceRef::GitRepo { .. } => "git_repo",
        };
        if let Err(err) = session
            .record_metric(MetricEvent::CourseOpened {
                source_kind: source_kind.to_owned(),
                language: pack.course.language,
            })
            .await
        {
            eprintln!("codelab warning: failed to record course_opened metric: {err}");
        }

        let workspace_meta = session.store().read_meta().map_err(AppError::from)?;
        let progress = session.store().read_progress().map_err(AppError::from)?;

        Ok(OpenedWorkspaceView {
            epoch: session.epoch(),
            workspace_meta,
            progress,
            course: pack.course,
            manifest: pack.manifest,
            checks: pack.checks,
            newly_hydrated_files: hydration.newly_hydrated,
            skipped_existing_files: hydration.skipped_existing,
            recovered_from_backup: session.open_report().recovered_from_backup.clone(),
        })
    }

    pub async fn reload_active_manual(
        &self,
        expected_epoch: SessionEpoch,
    ) -> Result<ParsedCourse, AppError> {
        let session = self.require_active_session(expected_epoch)?;
        let loaded = session.loaded_course().ok_or_else(|| {
            AppError::new(ErrorCode::WorkspaceNotOpen, "error.notReady")
                .with_detail("active session has no loaded course pack")
        })?;

        let reloaded = tokio::task::spawn_blocking(move || {
            let reloaded = reload_loaded_pack(&loaded)?;
            if matches!(loaded.source, CourseSourceRef::GitRepo { .. })
                && loaded.pack_fingerprint != reloaded.pack_fingerprint
            {
                return Err(AppError::new(
                    ErrorCode::ExternalPackUntrusted,
                    "error.externalPackUntrusted",
                )
                .with_param("source", reloaded.source_key.as_str())
                .with_detail(
                    "the Git course changed; inspect and reopen it to use the new version",
                ));
            }
            if matches!(loaded.source, CourseSourceRef::LocalPackDir { .. })
                && loaded.manifest.as_ref().map(|manifest| &manifest.course_id)
                    != reloaded
                        .manifest
                        .as_ref()
                        .map(|manifest| &manifest.course_id)
            {
                return Err(AppError::new(
                    ErrorCode::SchemaValidationFailed,
                    "error.schemaValidationFailed",
                )
                .with_param("file", "codelab.json")
                .with_param(
                    "detail",
                    "courseId changed; reopen the course in its own workspace",
                ));
            }
            Ok(reloaded)
        })
        .await
        .map_err(join_error)??;

        if let Some(first_issue) = reloaded.validation_issues.first() {
            return Err(AppError::new(
                ErrorCode::SchemaValidationFailed,
                "error.schemaValidationFailed",
            )
            .with_param("file", first_issue.file.as_str())
            .with_param("detail", first_issue.message.as_str())
            .with_detail(format!("{}: {}", first_issue.file, first_issue.message)));
        }

        let still_active = self.require_active_session(expected_epoch)?;
        let course = reloaded.course.clone();
        still_active.update_loaded_course(reloaded);
        Ok(course)
    }

    async fn load_course_source(
        &self,
        source: CourseSourceInput,
        git_config: &GitImportConfig,
    ) -> Result<LoadedCoursePack, AppError> {
        match source {
            CourseSourceInput::Builtin { course_id } => {
                let builtin_dir = self.builtin_courses_dir().ok_or_else(|| {
                    AppError::new(ErrorCode::SchemaValidationFailed, "error.notReady")
                        .with_detail("built-in courses directory is not configured")
                })?;
                tokio::task::spawn_blocking(move || load_builtin_pack(&builtin_dir, &course_id))
                    .await
                    .map_err(join_error)?
            }
            CourseSourceInput::LocalMarkdown { file_path } => {
                let path = PathBuf::from(file_path);
                tokio::task::spawn_blocking(move || load_local_markdown(&path))
                    .await
                    .map_err(join_error)?
            }
            CourseSourceInput::LocalPackDir { dir_path } => {
                let path = PathBuf::from(dir_path);
                tokio::task::spawn_blocking(move || load_local_pack_dir(&path))
                    .await
                    .map_err(join_error)?
            }
            CourseSourceInput::GitRepo {
                url,
                git_ref,
                refresh,
            } => {
                let validated_url = validate_git_url(&url)?;
                let validated_ref = match git_ref.as_deref() {
                    Some(raw_ref) => Some(validate_git_ref(raw_ref)?),
                    None => None,
                };
                let cache_root = self.global_store().paths().git_packs_cache_dir();
                let fetched = fetch_git_pack(
                    &cache_root,
                    &validated_url,
                    validated_ref.as_deref(),
                    refresh,
                    git_config,
                )
                .await?;
                tokio::task::spawn_blocking(move || {
                    load_git_pack_dir(
                        &fetched.cache_dir,
                        &validated_url,
                        validated_ref,
                        fetched.source_key,
                    )
                })
                .await
                .map_err(join_error)?
            }
        }
    }
}

fn join_error(err: tokio::task::JoinError) -> AppError {
    AppError::new(ErrorCode::Cancelled, "error.unknown")
        .with_param("detail", err.to_string())
        .with_detail(err.to_string())
}
