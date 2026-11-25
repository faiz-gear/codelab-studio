use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

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
            .args(["clone", "--quiet", "--no-local", "--"])
            .arg(&self.0)
            .arg(destination)
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
    fs::write(
        learner.join(".codelab/progress.json"),
        r#"{"schemaVersion":1,"data":{"chapters":{"stable":{"completionState":"completed"}}}}"#,
    )
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
    assert!(recent.workspace.is_some());
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
    allow_cleanup(&data);
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
