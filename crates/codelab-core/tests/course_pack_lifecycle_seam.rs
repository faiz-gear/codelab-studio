use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use codelab_core::{
    App, AppCommand, AppConfig, AppResponse, ErrorCode, GitFetchAdapter, OpenCourseRequest,
    TrustSource, TrustSourceKind, WorkspacePath,
};
use tempfile::TempDir;

const REMOTE: &str = "https://example.test/team/course.git";
const NORMALIZED_REMOTE: &str = "https://example.test/team/course";

struct LocalGit(PathBuf);

impl GitFetchAdapter for LocalGit {
    fn checkout(&self, url: &str, destination: &Path) -> io::Result<()> {
        assert_eq!(url, REMOTE);
        let status = isolated_git()
            // Match the production Git adapter. Git for Windows appends `/` to
            // destination arguments, which breaks verbatim Windows paths.
            .current_dir(destination)
            .args(["clone", "--quiet", "--no-local", "--"])
            .arg(&self.0)
            .arg(".")
            .status()?;
        if !status.success() {
            return Err(io::Error::other("fixture checkout failed"));
        }
        Ok(())
    }
}

fn isolated_git() -> Command {
    let mut command = Command::new("git");
    // Git hooks export the active repository and index.
    for (name, _) in std::env::vars_os() {
        if name.as_encoded_bytes().starts_with(b"GIT_") {
            command.env_remove(name);
        }
    }
    command
}

fn git(root: &Path, args: &[&str]) {
    let output = isolated_git()
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
}

fn commit_pack(root: &Path, heading: &str, check: &str, starter: &str) {
    fs::create_dir_all(root.join("starter")).unwrap();
    fs::create_dir_all(root.join("solutions/stable")).unwrap();
    fs::write(
        root.join("codelab.json"),
        r#"{"schemaVersion":"1.0","courseId":"fixture","title":"Git Pack","language":"Go"}"#,
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        format!("# Pack\n\n## {heading} {{#stable}}\n\n### ③ Check\n\nConfirm the result."),
    )
    .unwrap();
    fs::write(
        root.join(".codelab-checks.json"),
        format!(
            r#"{{"schemaVersion":"1.0","chapters":[{{"chapterId":"stable","checks":[{{"id":"one","title":"{check}","type":"self_check"}}]}}]}}"#
        ),
    )
    .unwrap();
    fs::write(root.join("starter/main.go"), starter).unwrap();
    let (added, removed) = if starter == "starter v1" {
        ("original_test.go", "new_test.go")
    } else {
        ("new_test.go", "original_test.go")
    };
    let _ = fs::remove_file(root.join("starter").join(removed));
    fs::write(root.join("starter").join(added), starter).unwrap();
    fs::write(root.join("solutions/stable/main.go"), check).unwrap();
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-qm",
            heading,
        ],
    );
}

#[test]
fn git_import_trust_and_refresh_keep_the_learner_workspace() {
    let fixture = TempDir::new().unwrap();
    let repo = fixture.path().join("repo");
    let data = fixture.path().join("data");
    let learner = fixture.path().join("learner");
    fs::create_dir(&repo).unwrap();
    fs::create_dir(&learner).unwrap();
    git(&repo, &["init", "-q"]);
    commit_pack(&repo, "Exercise 1", "Before", "starter v1");
    let open_app = || {
        App::open_with_git_fetcher(
            AppConfig {
                data_dir: data.clone(),
            },
            Arc::new(LocalGit(repo.clone())),
        )
        .unwrap()
    };
    let app = open_app();
    let AppResponse::LoadedCourse(course) = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::GitPack {
            url: REMOTE.into(),
        }))
        .unwrap()
    else {
        panic!("expected Course preview")
    };
    assert!(!course.trusted);
    assert_eq!(course.language, Some(codelab_core::CourseLanguage::Go));
    assert_eq!(course.chapters[0].chapter_id, "stable");
    assert_eq!(
        course.chapters[0].three_step.as_ref().unwrap().step3_checks[0].title,
        "Before"
    );

    let cached_manual = PathBuf::from(course.entry_markdown_path.unwrap());
    assert!(cached_manual.starts_with(fs::canonicalize(&data).unwrap()));
    assert!(
        fs::metadata(&cached_manual)
            .unwrap()
            .permissions()
            .readonly()
    );
    let bind = |path: &Path| AppCommand::BindWorkspace {
        path: path.to_string_lossy().into(),
    };
    assert_eq!(
        app.execute(bind(&learner)).unwrap_err().code,
        ErrorCode::CourseUntrusted
    );
    assert_eq!(
        app.execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
            path: cached_manual.parent().unwrap().to_string_lossy().into(),
        }))
        .unwrap_err()
        .code,
        ErrorCode::CoursePathInvalid
    );
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::GitRepo,
        source: " https://EXAMPLE.test/team/course.git/ ".into(),
    }))
    .unwrap();
    assert_eq!(
        app.execute(bind(cached_manual.parent().unwrap()))
            .unwrap_err()
            .code,
        ErrorCode::InvalidWorkspacePath
    );
    let AppResponse::WorkspaceBound(workspace) = app.execute(bind(&learner)).unwrap() else {
        panic!("expected Workspace")
    };
    assert_eq!(
        fs::read_to_string(learner.join("main.go")).unwrap(),
        "starter v1"
    );
    assert!(
        !fs::metadata(learner.join("main.go"))
            .unwrap()
            .permissions()
            .readonly()
    );
    fs::write(learner.join("main.go"), "learner edit").unwrap();
    app.execute(AppCommand::CompleteExercise(codelab_core::ExerciseTarget {
        workspace_id: workspace.id,
        chapter_id: "stable".into(),
    }))
    .unwrap();

    commit_pack(&repo, "Exercise 1 revised", "After", "starter v2");
    let AppResponse::LoadedCourse(refreshed) = app.execute(AppCommand::RefreshGitCourse).unwrap()
    else {
        panic!("expected refreshed Course")
    };
    assert!(refreshed.trusted);
    assert_eq!(refreshed.chapters[0].title, "Exercise 1 revised");
    assert_eq!(
        refreshed.chapters[0]
            .three_step
            .as_ref()
            .unwrap()
            .step3_checks[0]
            .title,
        "After"
    );
    assert_eq!(
        fs::read_to_string(learner.join("main.go")).unwrap(),
        "learner edit"
    );
    let AppResponse::WorkspaceComparisons(comparisons) = app
        .execute(AppCommand::PreviewChapterSolution {
            workspace_id: workspace.id,
            chapter_id: "stable".into(),
        })
        .unwrap()
    else {
        panic!("expected refreshed Reference Solution")
    };
    assert_eq!(comparisons.len(), 1);
    assert_eq!(comparisons[0].current.path, "main.go");
    assert_eq!(comparisons[0].proposed, "After");
    assert_starter(&app, workspace.id, "starter v1", "original_test.go");
    assert!(matches!(
        app.execute(AppCommand::ReadWorkspaceFile(WorkspacePath {
            workspace_id: workspace.id,
            path: "main.go".into()
        }))
        .unwrap(),
        AppResponse::WorkspaceFile(_)
    ));
    let AppResponse::Recents(recents) = app.execute(AppCommand::ListRecents).unwrap() else {
        panic!("expected recents")
    };
    assert_eq!(recents[0].completed_exercises, 1);
    let fresh = fixture.path().join("fresh");
    fs::create_dir(&fresh).unwrap();
    let AppResponse::WorkspaceBound(new_workspace) = app.execute(bind(&fresh)).unwrap() else {
        panic!("expected new Workspace")
    };
    assert_starter(&app, new_workspace.id, "starter v2", "new_test.go");
    assert_eq!(
        fs::read_to_string(fresh.join("main.go")).unwrap(),
        "starter v2"
    );
    // A binding written by a version without a snapshot key must remain readable.
    let binding = fresh.join(".codelab/workspace.json");
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&binding).unwrap()).unwrap();
    assert!(
        json["data"]
            .as_object_mut()
            .unwrap()
            .remove("gitStarterSnapshot")
            .is_some()
    );
    fs::write(binding, serde_json::to_vec(&json).unwrap()).unwrap();
    fs::write(repo.join("codelab.json"), "invalid manifest").unwrap();
    git(&repo, &["add", "codelab.json"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-qm",
            "Invalid upstream",
        ],
    );
    assert_eq!(
        app.execute(AppCommand::RefreshGitCourse).unwrap_err().code,
        ErrorCode::CourseManifestInvalid
    );
    let AppResponse::LoadedCourse(still_valid) =
        app.execute(AppCommand::ReloadActiveCourse).unwrap()
    else {
        panic!("expected last valid Git revision")
    };
    assert_eq!(still_valid.chapters[0].title, "Exercise 1 revised");
    drop(app);

    // Reopening a recent uses the cached snapshot and persisted trust even offline.
    fs::rename(&repo, fixture.path().join("offline")).unwrap();
    let reopened = open_app();
    let AppResponse::RecentWorkspaceOpened(recent) = reopened
        .execute(AppCommand::OpenRecentWorkspace {
            path: learner.to_string_lossy().into(),
        })
        .unwrap()
    else {
        panic!("expected recent Workspace")
    };
    assert!(recent.course.trusted);
    assert_starter(
        &reopened,
        recent.workspace.unwrap().id,
        "starter v1",
        "original_test.go",
    );
    assert_eq!(
        recent.course.source,
        codelab_core::CourseSource::GitPack {
            url: NORMALIZED_REMOTE.into()
        }
    );
    assert_eq!(recent.course.chapters[0].chapter_id, "stable");
    assert_eq!(
        fs::read_to_string(learner.join("main.go")).unwrap(),
        "learner edit"
    );
    let AppResponse::WorkspaceBound(legacy) = reopened.execute(bind(&fresh)).unwrap() else {
        panic!("expected legacy Workspace binding")
    };
    assert_starter(&reopened, legacy.id, "starter v2", "new_test.go");
    allow_cleanup(&data);
}

// Restore fixture permissions so TempDir can remove the cache on every platform.
fn allow_cleanup(directory: &Path) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() || kind.is_file() {
            let original = entry.metadata().unwrap().permissions();
            #[cfg(unix)]
            let permissions = {
                use std::os::unix::fs::PermissionsExt;
                fs::Permissions::from_mode(original.mode() | 0o200)
            };
            #[cfg(windows)]
            #[allow(
                clippy::permissions_set_readonly_false,
                reason = "Windows only clears the read-only attribute for fixture cleanup"
            )]
            let permissions = {
                let mut permissions = original;
                permissions.set_readonly(false);
                permissions
            };
            fs::set_permissions(entry.path(), permissions).unwrap();
        }
        if kind.is_dir() {
            allow_cleanup(&entry.path());
        }
    }
}

fn assert_starter(app: &App, workspace_id: u64, expected: &str, test_path: &str) {
    assert_eq!(
        app.execute(AppCommand::ListStarterFiles { workspace_id })
            .unwrap(),
        AppResponse::StarterFiles(vec!["main.go".into(), test_path.into()]),
    );
    let AppResponse::WorkspaceComparisons(files) = app
        .execute(AppCommand::PreviewStarterFile(WorkspacePath {
            workspace_id,
            path: "main.go".into(),
        }))
        .unwrap()
    else {
        panic!("expected Starter comparison")
    };
    assert_eq!(files[0].proposed, expected);
}

#[test]
fn a_slow_git_refresh_cannot_block_course_close_or_publish_afterward() {
    struct DelayedGit {
        git: LocalGit,
        pause: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
    }

    impl GitFetchAdapter for DelayedGit {
        fn checkout(&self, url: &str, destination: &Path) -> io::Result<()> {
            if let Some((started, resume)) = self.pause.lock().unwrap().take() {
                started.send(()).unwrap();
                resume.recv().unwrap();
            }
            self.git.checkout(url, destination)
        }
    }

    let temp = TempDir::new().unwrap();
    let repo = temp.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    commit_pack(&repo, "Before", "Original", "starter v1");
    let adapter = Arc::new(DelayedGit {
        git: LocalGit(repo.clone()),
        pause: Mutex::new(None),
    });
    let data_dir = temp.path().join("data");
    let app = App::open_with_git_fetcher(
        AppConfig {
            data_dir: data_dir.clone(),
        },
        adapter.clone(),
    )
    .unwrap();
    let open = AppCommand::OpenCourse(OpenCourseRequest::GitPack { url: REMOTE.into() });
    app.execute(open.clone()).unwrap();
    commit_pack(&repo, "After", "Updated", "starter v2");

    let (started, fetching) = mpsc::channel();
    let (resume, paused) = mpsc::channel();
    *adapter.pause.lock().unwrap() = Some((started, paused));
    std::thread::scope(|scope| {
        let refresh = scope.spawn(|| app.execute(AppCommand::RefreshGitCourse));
        fetching.recv_timeout(Duration::from_secs(5)).unwrap();
        let (closed, close_response) = mpsc::channel();
        let app = &app;
        scope.spawn(move || {
            closed
                .send(app.execute(AppCommand::CloseActiveCourse))
                .unwrap()
        });
        let closed_before_checkout_completed = close_response.recv_timeout(Duration::from_secs(5));
        resume.send(()).unwrap();
        assert_eq!(
            closed_before_checkout_completed.unwrap().unwrap(),
            AppResponse::CourseClosed
        );
        assert_eq!(
            refresh.join().unwrap().unwrap_err().code,
            ErrorCode::NoActiveGitCourse
        );
    });
    let AppResponse::LoadedCourse(course) = app.execute(open).unwrap() else {
        panic!("expected to reopen the cached Course Pack")
    };
    assert_eq!(course.chapters[0].title, "Before");
    allow_cleanup(&data_dir);
}

#[test]
fn author_edits_live_preview_and_starts_an_isolated_trial() {
    use codelab_core::{
        CourseDraftRequest, CourseLanguage, DraftEdit, DraftFilePath, DraftFileWrite,
        DraftPreviewRequest, WorkspaceFileContents, WorkspaceFileWrite,
    };

    let temp = TempDir::new().unwrap();
    let source = temp.path().join("pack");
    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    let AppResponse::CourseDraft(draft) = app
        .execute(AppCommand::CreateCourseDraft(CourseDraftRequest {
            path: source.to_string_lossy().into(),
            title: "Python Practice".into(),
            language: CourseLanguage::Python,
            reference_solution: true,
        }))
        .unwrap()
    else {
        panic!("expected draft")
    };
    assert!(
        draft
            .files
            .iter()
            .any(|file| file == "solutions/ex-1/main.py")
    );
    assert!(draft.preview.diagnostics.is_empty());
    let course = draft.preview.course.unwrap();
    assert_eq!(course.title, "Python Practice");
    let steps = course.chapters[0].three_step.as_ref().unwrap();
    assert_eq!(steps.step1_command_ids.len(), 1);
    assert_eq!(steps.step2_entry_file.as_deref(), Some("main.py"));
    assert_eq!(steps.step3_checks[0].id, "greeting");
    assert_eq!(
        app.execute(AppCommand::CreateCourseDraft(CourseDraftRequest {
            path: source.to_string_lossy().into(),
            title: "Would overwrite".into(),
            language: CourseLanguage::Go,
            reference_solution: false,
        }))
        .unwrap_err()
        .code,
        ErrorCode::DraftDirectoryNotEmpty
    );

    let target = |path: &str| DraftFilePath {
        draft_id: draft.id,
        path: path.into(),
    };
    let invalid_checks = r#"{"schemaVersion":"1.0","chapters":[{"chapterId":"ex-1","checks":[{"id":"x","title":"Verify","type":"wrong_kind"}]}]}"#;
    let manual = "# Practice\n\n## Exercise 1 revised {#ex-1}\n\n### ① Run\n\nRun the example.\n\n### ② Edit\n\nChange the greeting.\n\n### ③ Verify\n\nCheck the greeting.";
    let AppResponse::DraftPreview(preview) = app
        .execute(AppCommand::PreviewCourseDraft(DraftPreviewRequest {
            draft_id: draft.id,
            edits: vec![
                DraftEdit {
                    path: "README.md".into(),
                    text: manual.into(),
                },
                DraftEdit {
                    path: ".codelab-checks.json".into(),
                    text: invalid_checks.into(),
                },
            ],
        }))
        .unwrap()
    else {
        panic!("expected live preview")
    };
    assert_eq!(
        preview.course.unwrap().chapters[0].title,
        "Exercise 1 revised"
    );
    assert_eq!(preview.diagnostics[0].path, ".codelab-checks.json");
    assert_eq!(
        preview.diagnostics[0].error.code,
        ErrorCode::CourseChecksInvalid
    );
    assert_eq!(
        preview.diagnostics[0].error.details["field"],
        "chapters[0].checks[0].type"
    );
    for (check_type, suffix, field) in [
        ("command_test", "", "command"),
        ("http_assert", r#", "request":{"url":"  "}"#, "request.url"),
    ] {
        let invalid =
            invalid_checks.replace(r#""wrong_kind""#, &format!("\"{check_type}\"{suffix}"));
        let target = target(".codelab-checks.json");
        let AppResponse::DraftPreview(preview) = app
            .execute(AppCommand::PreviewCourseDraft(DraftPreviewRequest {
                draft_id: draft.id,
                edits: vec![DraftEdit {
                    path: target.path.clone(),
                    text: invalid.clone(),
                }],
            }))
            .unwrap()
        else {
            panic!("expected diagnostics")
        };
        let error = &preview.diagnostics[0].error;
        assert_eq!(error.details["reasonCode"], "EMPTY_REQUIRED_FIELD");
        assert_eq!(
            error.details["field"],
            format!("chapters[0].checks[0].{field}")
        );

        let checks_path = source.join(&target.path);
        let original = fs::read(&checks_path).unwrap();
        fs::write(&checks_path, invalid).unwrap();
        assert_eq!(
            app.execute(AppCommand::StartTrialWorkspace { draft_id: draft.id })
                .unwrap_err()
                .code,
            ErrorCode::CourseChecksInvalid
        );
        fs::write(checks_path, original).unwrap();
    }
    assert!(
        !fs::read_to_string(source.join("README.md"))
            .unwrap()
            .contains("revised")
    );

    let AppResponse::WorkspaceFile(file) = app
        .execute(AppCommand::ReadCourseDraftFile(target("README.md")))
        .unwrap()
    else {
        panic!("expected manual source")
    };
    app.execute(AppCommand::WriteCourseDraftFile(DraftFileWrite {
        target: target("README.md"),
        text: manual.into(),
        expected_revision: file.revision,
    }))
    .unwrap();
    let AppResponse::TrialWorkspaceOpened(trial) = app
        .execute(AppCommand::StartTrialWorkspace { draft_id: draft.id })
        .unwrap()
    else {
        panic!("expected Trial Workspace")
    };
    assert!(trial.course.trusted);
    assert_eq!(trial.course.chapters[0].title, "Exercise 1 revised");
    let trial_path = Path::new(&trial.workspace.root);
    assert!(!trial_path.starts_with(&source));
    let file_target = WorkspacePath {
        workspace_id: trial.workspace.id,
        path: "main.py".into(),
    };
    let AppResponse::WorkspaceFile(starter) = app
        .execute(AppCommand::ReadWorkspaceFile(file_target.clone()))
        .unwrap()
    else {
        panic!("expected Writable Trial Workspace file")
    };
    assert!(matches!(
        starter.contents,
        WorkspaceFileContents::Text { .. }
    ));
    app.execute(AppCommand::WriteWorkspaceFile(WorkspaceFileWrite {
        target: file_target,
        text: "trial edit".into(),
        expected_revision: starter.revision,
    }))
    .unwrap();
    assert_eq!(
        fs::read_to_string(trial_path.join("main.py")).unwrap(),
        "trial edit"
    );
    assert_ne!(
        fs::read_to_string(source.join("starter/main.py")).unwrap(),
        "trial edit"
    );
    assert_eq!(
        app.execute(AppCommand::BindWorkspace {
            path: source.to_string_lossy().into()
        })
        .unwrap_err()
        .code,
        ErrorCode::InvalidWorkspacePath
    );
    assert!(!source.join(".codelab").exists());

    app.execute(AppCommand::CloseActiveCourse).unwrap();
    assert!(!trial_path.exists());
    let AppResponse::Recents(recents) = app.execute(AppCommand::ListRecents).unwrap() else {
        panic!("expected recents")
    };
    assert!(recents.is_empty());
    assert!(matches!(
        app.execute(AppCommand::ReadCourseDraftFile(target("README.md")))
            .unwrap(),
        AppResponse::WorkspaceFile(_)
    ));
}

#[test]
fn opening_an_existing_draft_requires_registered_trust_for_trial_and_honors_revocation() {
    use codelab_core::{AppEvent, CourseDraftRequest, CourseLanguage, WorkspaceTerminalRequest};

    let temp = TempDir::new().unwrap();
    let source = temp.path().join("pack");
    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    app.execute(AppCommand::CreateCourseDraft(CourseDraftRequest {
        path: source.to_string_lossy().into(),
        title: "External draft".into(),
        language: CourseLanguage::Go,
        reference_solution: false,
    }))
    .unwrap();
    let AppResponse::CourseDraft(draft) = app
        .execute(AppCommand::OpenCourseDraft {
            path: source.to_string_lossy().into(),
        })
        .unwrap()
    else {
        panic!("expected existing draft")
    };
    let start = AppCommand::StartTrialWorkspace { draft_id: draft.id };
    let trust = TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: draft.root,
    };
    assert_eq!(
        app.execute(start.clone()).unwrap(),
        AppResponse::TrialTrustRequired(trust.clone())
    );
    assert_eq!(
        app.execute(AppCommand::ListTrustEntries).unwrap(),
        AppResponse::TrustEntries(vec![])
    );
    app.execute(AppCommand::GrantTrust(trust.clone())).unwrap();
    let AppResponse::TrialWorkspaceOpened(trial) = app.execute(start.clone()).unwrap() else {
        panic!("expected trusted Trial Workspace")
    };
    assert!(trial.course.trusted);
    let terminal = AppCommand::OpenTerminal(WorkspaceTerminalRequest {
        workspace_id: trial.workspace.id,
    });
    assert!(matches!(
        app.execute(terminal.clone()).unwrap(),
        AppResponse::TerminalOpened(_)
    ));
    let events = app.subscribe();
    app.execute(AppCommand::RevokeTrust(trust.clone())).unwrap();
    let drained: Vec<_> = std::iter::from_fn(|| events.try_recv().ok()).collect();
    let trust_event = drained
        .iter()
        .find(|event| matches!(event, AppEvent::CourseTrustChanged(_)))
        .unwrap_or_else(|| panic!("expected trust revocation event, saw: {drained:?}"));
    assert_eq!(
        trust_event,
        &AppEvent::CourseTrustChanged(codelab_core::CourseTrustChanged {
            source: trial.course.source,
            trusted: false,
        })
    );
    assert_eq!(
        app.execute(terminal).unwrap_err().code,
        ErrorCode::CourseUntrusted
    );
    assert_eq!(
        app.execute(start).unwrap(),
        AppResponse::TrialTrustRequired(trust)
    );
}
