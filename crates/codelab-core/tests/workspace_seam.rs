use std::fs;
use std::path::Path;

use codelab_core::{
    App, AppCommand, AppConfig, AppEvent, AppResponse, ErrorCode, MAX_WORKSPACE_TEXT_BYTES,
    OpenCourseRequest, StoreKind, TrustSource, TrustSourceKind, WorkspaceFileContents,
};
use tempfile::TempDir;

fn path(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

fn open_app(data: &Path) -> App {
    App::open(AppConfig {
        data_dir: data.into(),
    })
    .unwrap()
}

fn open_course(app: &App, request: OpenCourseRequest) -> bool {
    let AppResponse::LoadedCourse(course) = app.execute(AppCommand::OpenCourse(request)).unwrap()
    else {
        panic!("expected loaded Course");
    };
    course.trusted
}

fn builtin() -> OpenCourseRequest {
    OpenCourseRequest::Builtin {
        course_id: "go-goframe-practice".into(),
    }
}

fn bind(app: &App, root: &Path) -> Result<AppResponse, codelab_core::DomainError> {
    app.execute(AppCommand::BindWorkspace { path: path(root) })
}

#[test]
fn local_course_requires_global_trust_before_binding_and_can_be_revoked() {
    let temp = TempDir::new().unwrap();
    let data = temp.path().join("app-data");
    let root = temp.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let pack = temp.path().join("pack");
    fs::create_dir(&pack).unwrap();
    fs::write(pack.join("README.md"), "# Local\n\n## Exercise 1").unwrap();
    fs::write(
        pack.join("codelab.json"),
        r#"{"schemaVersion":"1.0","courseId":"local","title":"Local","trusted":true}"#,
    )
    .unwrap();
    let local = OpenCourseRequest::LocalPack { path: path(&pack) };
    let trust_source = TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: path(&pack.join(".")),
    };

    let app = open_app(&data);
    assert!(!open_course(&app, local.clone()));
    fs::create_dir(root.join(".codelab")).unwrap();
    fs::write(root.join(".codelab/trust.json"), r#"{"trusted":true}"#).unwrap();
    assert_eq!(
        bind(&app, &root).unwrap_err().code,
        ErrorCode::CourseUntrusted
    );
    assert!(!root.join(".codelab/workspace.json").exists());

    app.execute(AppCommand::GrantTrust(trust_source.clone()))
        .unwrap();
    assert!(matches!(
        bind(&app, &root).unwrap(),
        AppResponse::WorkspaceBound(_)
    ));
    drop(app);

    let app = open_app(&data);
    assert!(open_course(&app, local.clone()));
    app.execute(AppCommand::RevokeTrust(trust_source)).unwrap();
    assert_eq!(
        bind(&app, &root).unwrap_err().code,
        ErrorCode::CourseUntrusted
    );
    assert!(!open_course(&app, local));
    assert!(open_course(&app, builtin()));
}

fn workspace_id(app: &App, root: &Path) -> u64 {
    let AppResponse::WorkspaceBound(workspace) = bind(app, root).unwrap() else {
        panic!("expected bound Workspace");
    };
    workspace.id
}

fn file_target(workspace_id: u64, path: &str) -> codelab_core::WorkspacePath {
    codelab_core::WorkspacePath {
        workspace_id,
        path: path.into(),
    }
}

fn read_file(app: &App, workspace_id: u64, path: &str) -> codelab_core::WorkspaceFile {
    let AppResponse::WorkspaceFile(file) = app
        .execute(AppCommand::ReadWorkspaceFile(file_target(
            workspace_id,
            path,
        )))
        .unwrap()
    else {
        panic!("expected Workspace file");
    };
    file
}

fn write_file(workspace_id: u64, path: &str, text: &str, revision: Option<String>) -> AppCommand {
    AppCommand::WriteWorkspaceFile(codelab_core::WorkspaceFileWrite {
        target: file_target(workspace_id, path),
        text: text.into(),
        expected_revision: revision,
    })
}

fn make_local_pack(root: &Path) -> TrustSource {
    fs::create_dir_all(root.join("template/nested")).unwrap();
    fs::write(root.join("template/nested/code.go"), "starter").unwrap();
    fs::write(root.join("template/nested/code_test.go"), "visible tests").unwrap();
    fs::write(root.join("README.md"), "# Local\n\n## Exercise 1").unwrap();
    fs::write(
        root.join("codelab.json"),
        r#"{"schemaVersion":"1.0","courseId":"local","title":"Local","starterDir":"template"}"#,
    )
    .unwrap();
    TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: path(root),
    }
}

#[test]
fn starter_seeds_only_empty_workspaces_and_reload_preserves_source_code() {
    let temp = TempDir::new().unwrap();
    let pack = temp.path().join("course");
    let source = make_local_pack(&pack);
    let data = temp.path().join("app-data");
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let existing = temp.path().join("existing");
    for root in [&first, &second, &existing] {
        fs::create_dir(root).unwrap();
    }
    fs::create_dir(existing.join("nested")).unwrap();
    fs::write(existing.join("nested/code.go"), "existing work").unwrap();

    let app = open_app(&data);
    app.execute(AppCommand::GrantTrust(source)).unwrap();
    assert!(open_course(
        &app,
        OpenCourseRequest::LocalPack { path: path(&pack) }
    ));
    let first_id = workspace_id(&app, &first);
    assert_eq!(
        fs::read_to_string(first.join("nested/code_test.go")).unwrap(),
        "visible tests"
    );

    let file = read_file(&app, first_id, "nested/code.go");
    app.execute(write_file(
        first_id,
        &file.path,
        "learner edit",
        file.revision,
    ))
    .unwrap();
    fs::write(pack.join("template/nested/code.go"), "upstream edit").unwrap();
    fs::write(
        pack.join("README.md"),
        "# Local\n\n## Exercise 1\n\nUpdated",
    )
    .unwrap();
    app.execute(AppCommand::ReloadActiveCourse).unwrap();
    assert_eq!(
        read_file(&app, first_id, "nested/code.go").contents,
        WorkspaceFileContents::Text {
            text: "learner edit".into()
        }
    );

    let second_id = workspace_id(&app, &second);
    assert_ne!(first_id, second_id);
    assert_eq!(
        fs::read_to_string(second.join("nested/code.go")).unwrap(),
        "upstream edit"
    );
    assert_eq!(
        app.execute(write_file(first_id, "nested/code.go", "stale", None))
            .unwrap_err()
            .code,
        ErrorCode::NoActiveWorkspace
    );
    workspace_id(&app, &existing);
    assert_eq!(
        fs::read_to_string(existing.join("nested/code.go")).unwrap(),
        "existing work"
    );
    assert!(!existing.join("nested/code_test.go").exists());
    assert_eq!(
        bind(&app, &pack).unwrap_err().code,
        ErrorCode::InvalidWorkspacePath
    );

    drop(app);
    let app = open_app(&data);
    let AppResponse::RecentWorkspaceOpened(recent) = app
        .execute(AppCommand::OpenRecentWorkspace { path: path(&first) })
        .unwrap()
    else {
        panic!("expected recent Workspace");
    };
    assert_eq!(recent.course.course_id, "local");
    assert_eq!(
        recent.workspace.unwrap().root,
        path(&fs::canonicalize(&first).unwrap())
    );
    let AppResponse::Recents(recents) = app.execute(AppCommand::ListRecents).unwrap() else {
        panic!("expected recents");
    };
    assert_eq!(
        recents[0].workspace_path,
        path(&fs::canonicalize(first).unwrap())
    );
}

#[test]
fn local_markdown_can_share_a_directory_with_learner_files() {
    let temp = TempDir::new().unwrap();
    let app = open_app(&temp.path().join("app-data"));
    let root = temp.path().join("notes");
    fs::create_dir(&root).unwrap();
    let markdown = root.join("course.md");
    fs::write(&markdown, "# Course\n\n## Guide").unwrap();
    let request = OpenCourseRequest::LocalMarkdown {
        path: path(&markdown),
    };
    assert!(!open_course(&app, request));
    let source = TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: path(&markdown),
    };
    app.execute(AppCommand::GrantTrust(source.clone())).unwrap();
    let id = workspace_id(&app, &root);
    app.execute(AppCommand::CreateWorkspaceFile(file_target(
        id,
        "draft.txt",
    )))
    .unwrap();
    assert_eq!(
        app.execute(AppCommand::ReadWorkspaceFile(file_target(id, "course.md")))
            .unwrap_err()
            .code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    app.execute(AppCommand::RevokeTrust(source)).unwrap();
    for command in [
        AppCommand::ReadWorkspaceFile(file_target(id, "draft.txt")),
        write_file(id, "draft.txt", "edit", None),
        AppCommand::ListWorkspaceFiles(file_target(id, "")),
        AppCommand::OpenWorkspaceIde(codelab_core::WorkspaceIdeRequest {
            target: file_target(id, "draft.txt"),
            ide: codelab_core::ExternalIde::VsCode,
        }),
    ] {
        assert_eq!(
            app.execute(command).unwrap_err().code,
            ErrorCode::CourseUntrusted
        );
    }
}

#[test]
fn reports_non_text_and_conflicts_without_overwriting_new_disk_content() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("text.txt"), "original").unwrap();
    fs::write(root.join("allowed-controls.txt"), "a\tb\r\nc\x0cd").unwrap();
    fs::write(
        root.join("exact-limit.txt"),
        vec![b'x'; MAX_WORKSPACE_TEXT_BYTES],
    )
    .unwrap();
    fs::write(root.join("binary"), b"NUL\0data").unwrap();
    fs::write(root.join("control"), b"bad\x01byte").unwrap();
    fs::write(root.join("invalid-utf8"), [0xff, 0xfe]).unwrap();
    fs::write(root.join("large"), vec![b'a'; MAX_WORKSPACE_TEXT_BYTES + 1]).unwrap();
    let app = open_app(&temp.path().join("app-data"));
    open_course(&app, builtin());
    let id = workspace_id(&app, &root);
    let allowed = read_file(&app, id, "allowed-controls.txt");
    assert_eq!(
        allowed.contents,
        WorkspaceFileContents::Text {
            text: "a\tb\r\nc\x0cd".into()
        }
    );
    let exact = read_file(&app, id, "exact-limit.txt");
    assert_eq!(exact.size_bytes, MAX_WORKSPACE_TEXT_BYTES as u64);
    assert!(matches!(exact.contents, WorkspaceFileContents::Text { .. }));
    let binary = read_file(&app, id, "binary");
    assert_eq!(binary.contents, WorkspaceFileContents::Binary);
    assert_eq!(binary.size_bytes, 8);
    assert_eq!(
        read_file(&app, id, "control").contents,
        WorkspaceFileContents::Binary
    );
    assert_eq!(
        read_file(&app, id, "invalid-utf8").contents,
        WorkspaceFileContents::Binary
    );
    let large = read_file(&app, id, "large");
    assert_eq!(large.contents, WorkspaceFileContents::TooLarge);
    assert_eq!(large.size_bytes, MAX_WORKSPACE_TEXT_BYTES as u64 + 1);
    assert_eq!(
        app.execute(write_file(id, "binary", "replace", binary.revision))
            .unwrap_err()
            .code,
        ErrorCode::WorkspaceFileNotText
    );
    assert_eq!(
        app.execute(write_file(id, "large", "replace", large.revision))
            .unwrap_err()
            .code,
        ErrorCode::WorkspaceFileNotText
    );

    let baseline = read_file(&app, id, &path(&root.join("text.txt")));
    assert_eq!(
        app.execute(write_file(
            id,
            "text.txt",
            "bad\x01draft",
            baseline.revision.clone()
        ))
        .unwrap_err()
        .code,
        ErrorCode::WorkspaceFileNotText
    );
    assert_eq!(
        app.execute(write_file(
            id,
            "text.txt",
            &"z".repeat(MAX_WORKSPACE_TEXT_BYTES + 1),
            baseline.revision.clone()
        ))
        .unwrap_err()
        .code,
        ErrorCode::WorkspaceFileNotText
    );
    assert_eq!(
        fs::read_to_string(root.join("text.txt")).unwrap(),
        "original"
    );
    fs::write(root.join("text.txt"), "external").unwrap();
    let AppResponse::WorkspaceFileConflict(disk) = app
        .execute(write_file(id, "text.txt", "my draft", baseline.revision))
        .unwrap()
    else {
        panic!("expected a save conflict");
    };
    assert_eq!(
        disk.contents,
        WorkspaceFileContents::Text {
            text: "external".into()
        }
    );
    assert_eq!(
        fs::read_to_string(root.join("text.txt")).unwrap(),
        "external"
    );

    app.execute(write_file(id, "text.txt", "my draft", disk.revision))
        .unwrap();
    let file = read_file(&app, id, "text.txt");
    fs::remove_file(root.join("text.txt")).unwrap();
    assert!(matches!(
        app.execute(write_file(id, "text.txt", "kept", file.revision)).unwrap(),
        AppResponse::WorkspaceFileConflict(file) if file.contents == WorkspaceFileContents::Missing
    ));
    assert!(!root.join("text.txt").exists());
    app.execute(write_file(id, "text.txt", "kept", None))
        .unwrap();
    assert_eq!(fs::read_to_string(root.join("text.txt")).unwrap(), "kept");
}

#[cfg(unix)]
#[test]
fn saving_a_revision_conflicts_when_the_path_resolves_to_a_different_file_with_the_same_bytes() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    let original = root.join("original.txt");
    let moved_original = root.join("moved-original.txt");
    let other = root.join("other.txt");
    let shared_text = "same file contents";
    fs::write(&original, shared_text).unwrap();
    fs::write(&other, shared_text).unwrap();
    let app = open_app(&temp.path().join("app-data"));
    open_course(&app, builtin());
    let id = workspace_id(&app, &root);
    let opened = read_file(&app, id, "original.txt");

    fs::rename(&original, &moved_original).unwrap();
    symlink("other.txt", &original).unwrap();

    let result = app
        .execute(write_file(
            id,
            &opened.path,
            "learner draft",
            opened.revision,
        ))
        .unwrap();
    let AppResponse::WorkspaceFileConflict(disk) = result else {
        panic!("expected a save conflict after the path changed targets, got {result:?}");
    };
    assert_eq!(disk.path, "other.txt");
    assert_eq!(fs::read_to_string(&other).unwrap(), shared_text);
    assert_eq!(fs::read_to_string(&moved_original).unwrap(), shared_text);
}

#[test]
fn file_creation_and_rename_cannot_replace_an_existing_target() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("keep.txt"), "existing").unwrap();
    let app = open_app(&temp.path().join("app-data"));
    open_course(&app, builtin());
    let id = workspace_id(&app, &root);
    let source = file_target(id, "new.txt");
    assert_eq!(
        app.execute(AppCommand::CreateWorkspaceFile(file_target(id, "keep.txt")))
            .unwrap_err()
            .code,
        ErrorCode::WorkspaceFileExists
    );
    app.execute(AppCommand::CreateWorkspaceFile(source.clone()))
        .unwrap();
    let rename = |new_path: &str| {
        AppCommand::RenameWorkspaceFile(codelab_core::WorkspaceFileRename {
            target: source.clone(),
            new_path: new_path.into(),
        })
    };
    assert_eq!(
        app.execute(rename("keep.txt")).unwrap_err().code,
        ErrorCode::WorkspaceFileExists
    );
    assert!(root.join("new.txt").exists());
    app.execute(rename("renamed.txt")).unwrap();
    assert!(root.join("renamed.txt").exists());
    assert!(!root.join("new.txt").exists());
    assert_eq!(
        fs::read_to_string(root.join("keep.txt")).unwrap(),
        "existing"
    );
}

#[test]
fn rejects_parent_traversal_absolute_escape_and_hidden_metadata() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("safe.txt"), "learner").unwrap();
    let adjacent = temp.path().join("work-sibling");
    fs::create_dir(&adjacent).unwrap();
    fs::write(adjacent.join("outside.txt"), "outside").unwrap();
    let app = open_app(&temp.path().join("app-data"));
    open_course(&app, builtin());
    let id = workspace_id(&app, &root);

    for bad_path in [
        "../work-sibling/outside.txt",
        "nested/../safe.txt",
        &path(&adjacent.join("outside.txt")),
        ".codelab/workspace.json",
        ".env",
        "deployment-secret.txt",
    ] {
        for command in [
            AppCommand::ReadWorkspaceFile(file_target(id, bad_path)),
            AppCommand::CreateWorkspaceFile(file_target(id, bad_path)),
            write_file(id, bad_path, "changed", None),
            AppCommand::RenameWorkspaceFile(codelab_core::WorkspaceFileRename {
                target: file_target(id, "safe.txt"),
                new_path: bad_path.into(),
            }),
        ] {
            assert_eq!(
                app.execute(command).unwrap_err().code,
                ErrorCode::WorkspacePathOutsideRoot,
                "{bad_path}"
            );
        }
    }
    assert_eq!(
        fs::read_to_string(adjacent.join("outside.txt")).unwrap(),
        "outside"
    );
    assert_eq!(
        fs::read_to_string(root.join("safe.txt")).unwrap(),
        "learner"
    );
    assert_eq!(
        bind(&app, temp.path()).unwrap_err().code,
        ErrorCode::InvalidWorkspacePath
    );
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_redirect_starter_metadata_or_file_operations_outside_the_root() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let pack = temp.path().join("course");
    let source = make_local_pack(&pack);
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("private.txt"), "private").unwrap();

    symlink(outside.join("private.txt"), pack.join("template/escape")).unwrap();
    let app = open_app(&temp.path().join("app-data"));
    app.execute(AppCommand::GrantTrust(source)).unwrap();
    open_course(&app, OpenCourseRequest::LocalPack { path: path(&pack) });
    assert_eq!(
        bind(&app, &root).unwrap_err().code,
        ErrorCode::WorkspaceStarterFailed
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    fs::remove_file(pack.join("template/escape")).unwrap();

    symlink(&outside, root.join(".codelab")).unwrap();
    assert_eq!(
        bind(&app, &root).unwrap_err().code,
        ErrorCode::StorageReadFailed
    );
    assert!(!outside.join("workspace.json").exists());
    fs::remove_file(root.join(".codelab")).unwrap();

    let id = workspace_id(&app, &root);
    symlink(&outside, root.join("escape")).unwrap();
    symlink(outside.join("private.txt"), root.join("escape.txt")).unwrap();
    assert_eq!(
        app.execute(AppCommand::ReadWorkspaceFile(file_target(id, "escape.txt")))
            .unwrap_err()
            .code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(
        app.execute(AppCommand::CreateWorkspaceFile(file_target(
            id,
            "escape/new.txt"
        )))
        .unwrap_err()
        .code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(
        app.execute(write_file(id, "escape/private.txt", "modified", None))
            .unwrap_err()
            .code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(
        app.execute(AppCommand::RenameWorkspaceFile(
            codelab_core::WorkspaceFileRename {
                target: file_target(id, "nested/code.go"),
                new_path: "escape/new.txt".into(),
            }
        ))
        .unwrap_err()
        .code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(
        fs::read_to_string(outside.join("private.txt")).unwrap(),
        "private"
    );
    assert!(!outside.join("new.txt").exists());

    let missing_outside = outside.join("never-created.txt");
    symlink(&missing_outside, root.join("dangling.txt")).unwrap();
    for command in [
        AppCommand::ReadWorkspaceFile(file_target(id, "dangling.txt")),
        write_file(id, "dangling.txt", "recreated", None),
        AppCommand::ReadWorkspaceFile(file_target(id, "escape/missing.txt")),
        write_file(id, "escape/missing.txt", "recreated", None),
    ] {
        assert_eq!(
            app.execute(command).unwrap_err().code,
            ErrorCode::WorkspacePathOutsideRoot
        );
    }
    assert!(!missing_outside.exists());
    assert!(!outside.join("missing.txt").exists());
}

#[test]
fn watcher_reports_disk_changes_within_the_debounce_budget_and_hides_exclusions() {
    use std::time::{Duration, Instant};

    let temp = TempDir::new().unwrap();
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    for excluded in [".git", "node_modules", ".codelab"] {
        fs::create_dir(root.join(excluded)).unwrap();
    }
    let app = open_app(&temp.path().join("app-data"));
    open_course(&app, builtin());
    let id = workspace_id(&app, &root);
    let events = app.subscribe();
    let await_path = |label: &str| {
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            let event = events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("Workspace change within 500ms");
            if let AppEvent::WorkspaceFilesChanged(event) = event {
                assert_eq!(event.workspace_id, id);
                assert!(event.paths.iter().all(|path| !path.starts_with(".git/")
                    && !path.starts_with("node_modules/")
                    && !path.starts_with(".codelab/")));
                if event.paths.iter().any(|path| path == label) {
                    break;
                }
            }
        }
    };

    for excluded in [".git", "node_modules", ".codelab"] {
        fs::write(root.join(excluded).join("noise"), "ignored").unwrap();
    }
    fs::write(root.join("live.txt"), "one").unwrap();
    await_path("live.txt");
    fs::write(root.join("live.txt"), "two").unwrap();
    await_path("live.txt");
    fs::remove_file(root.join("live.txt")).unwrap();
    await_path("live.txt");
    fs::create_dir(root.join("new-directory")).unwrap();
    await_path("new-directory");
    fs::write(root.join("new-directory/new-file"), "nested").unwrap();
    await_path("new-directory/new-file");
    assert_eq!(
        read_file(&app, id, "live.txt").contents,
        WorkspaceFileContents::Missing
    );
}

#[test]
fn restores_workspace_metadata_and_quarantines_corruption_on_reopen() {
    let temp = TempDir::new().unwrap();
    let data = temp.path().join("app-data");
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    let app = open_app(&data);
    open_course(&app, builtin());
    workspace_id(&app, &root);
    assert!(root.join("task-api/main_test.go").is_file());
    assert!(root.join("task-api-gf/main_test.go").is_file());

    let metadata = root.join(".codelab");
    let corrupt = [
        ("workspace.json", b"\xff".as_slice(), StoreKind::Workspace),
        (
            "progress.json",
            br#"{"schemaVersion":42,"data":{}}"#.as_slice(),
            StoreKind::Progress,
        ),
        (
            "metrics.json",
            br#"{"schemaVersion":1,"data":{"courseOpened":-1}}"#.as_slice(),
            StoreKind::Metrics,
        ),
    ];
    drop(app);
    for (name, contents, _) in corrupt {
        fs::write(metadata.join(name), contents).unwrap();
    }
    let app = open_app(&data);
    let AppResponse::RecentWorkspaceOpened(reopened) = app
        .execute(AppCommand::OpenRecentWorkspace { path: path(&root) })
        .unwrap()
    else {
        panic!("expected recent Workspace");
    };
    let recovered = reopened.workspace.unwrap().recovery_notices;
    for (name, original, kind) in corrupt {
        let notice = recovered
            .iter()
            .find(|notice| notice.store == kind)
            .unwrap();
        assert_eq!(notice.error_code, ErrorCode::StorageCorrupted);
        assert_eq!(
            fs::read(notice.backup_path.as_ref().unwrap()).unwrap(),
            original
        );
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(metadata.join(name)).unwrap()).unwrap();
        assert_eq!(json["schemaVersion"], 1);
    }
    let AppResponse::WorkspaceBound(again) = bind(&app, &root).unwrap() else {
        panic!("expected bound Workspace");
    };
    assert!(again.recovery_notices.is_empty());
}

#[test]
fn rebinding_preserves_completed_chapters_and_check_metadata() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    let app = open_app(&temp.path().join("app-data"));
    open_course(&app, builtin());
    workspace_id(&app, &root);
    let progress = root.join(".codelab/progress.json");
    let completed = br#"{"schemaVersion":1,"data":{"chapters":{"ex-0":{"completionState":"completed","lastCheckResult":{"passed":false}}}}}"#;
    fs::write(&progress, completed).unwrap();
    workspace_id(&app, &root);
    let AppResponse::Recents(recents) = app.execute(AppCommand::ListRecents).unwrap() else {
        panic!("expected recents");
    };
    assert_eq!(recents[0].completed_exercises, 1);
    assert_eq!(fs::read(progress).unwrap(), completed);
}

#[cfg(unix)]
#[test]
fn revocation_removes_the_original_grant_after_the_source_path_is_redirected() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let pack = temp.path().join("pack");
    let other = temp.path().join("other");
    let source = make_local_pack(&pack);
    let other_source = make_local_pack(&other);
    let app = open_app(&temp.path().join("app-data"));
    app.execute(AppCommand::GrantTrust(source)).unwrap();
    app.execute(AppCommand::GrantTrust(other_source)).unwrap();
    let pack = fs::canonicalize(pack).unwrap();
    let source = TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: path(&pack),
    };
    assert!(open_course(
        &app,
        OpenCourseRequest::LocalPack { path: path(&pack) }
    ));
    fs::rename(&pack, temp.path().join("previous")).unwrap();
    symlink(&other, &pack).unwrap();

    let root = temp.path().join("work");
    fs::create_dir(&root).unwrap();
    assert_eq!(
        bind(&app, &root).unwrap_err().code,
        ErrorCode::WorkspaceStarterFailed
    );
    app.execute(AppCommand::RevokeTrust(source)).unwrap();
    assert_eq!(
        bind(&app, &root).unwrap_err().code,
        ErrorCode::CourseUntrusted
    );
    assert!(open_course(
        &app,
        OpenCourseRequest::LocalPack { path: path(&other) }
    ));
}
