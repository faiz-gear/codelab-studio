use std::fs;
use std::path::Path;

use codelab_core::{
    App, AppCommand, AppConfig, AppResponse, ErrorCode, MAX_WORKSPACE_TEXT_BYTES,
    OpenCourseRequest, TrustSource, TrustSourceKind, WorkspaceComparison, WorkspaceFileContents,
    WorkspaceFileWrite, WorkspacePath,
};
use tempfile::TempDir;

fn open(pack: &Path, request: OpenCourseRequest) -> (App, TempDir, u64) {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("workspace");
    fs::create_dir(&root).unwrap();
    let app = App::open(AppConfig {
        data_dir: temp.path().join("data"),
    })
    .unwrap();
    app.execute(AppCommand::GrantTrust(TrustSource {
        source_kind: TrustSourceKind::LocalPath,
        source: pack.to_str().unwrap().into(),
    }))
    .unwrap();
    app.execute(AppCommand::OpenCourse(request)).unwrap();
    let AppResponse::WorkspaceBound(workspace) = app
        .execute(AppCommand::BindWorkspace {
            path: root.to_str().unwrap().into(),
        })
        .unwrap()
    else {
        panic!("expected a bound Workspace");
    };
    (app, temp, workspace.id)
}

fn preview(app: &App, workspace_id: u64, chapter_id: &str) -> Vec<WorkspaceComparison> {
    let AppResponse::WorkspaceComparisons(files) = app
        .execute(AppCommand::PreviewChapterSolution {
            workspace_id,
            chapter_id: chapter_id.into(),
        })
        .unwrap()
    else {
        panic!("expected Workspace comparisons");
    };
    files
}

fn target(workspace_id: u64, path: &str) -> WorkspacePath {
    WorkspacePath {
        workspace_id,
        path: path.into(),
    }
}

fn apply(app: &App, workspace_id: u64, comparison: &WorkspaceComparison) -> AppResponse {
    app.execute(AppCommand::ApplyWorkspaceComparison(WorkspaceFileWrite {
        target: target(workspace_id, &comparison.current.path),
        text: comparison.proposed.clone(),
        expected_revision: comparison.current.revision.clone(),
    }))
    .unwrap()
}

#[test]
fn compares_chapter_snapshots_and_applies_only_the_confirmed_file() {
    let pack = TempDir::new().unwrap();
    let root = pack.path();
    fs::write(
        root.join("codelab.json"),
        r#"{
        "schemaVersion":"1.0","courseId":"sample","title":"Sample",
        "chapters":[{"chapterId":"stable-id","match":{"headingPrefix":"Renamed"}}]
    }"#,
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "# Sample\n\n## Renamed heading\n\n```go file=hint.go\n// ignored\n```\n",
    )
    .unwrap();
    let snapshot = root.join("solutions/stable-id");
    fs::create_dir_all(snapshot.join("nested")).unwrap();
    fs::write(snapshot.join("main.go"), "package main\n").unwrap();
    fs::write(snapshot.join("nested/new.go"), "package nested\n").unwrap();

    let (app, temp, id) = open(
        root,
        OpenCourseRequest::LocalPack {
            path: root.to_str().unwrap().into(),
        },
    );
    let workspace = temp.path().join("workspace");
    let draft = "package main\n\n  // my partial implementation\n";
    fs::write(workspace.join("main.go"), draft).unwrap();

    let comparisons = preview(&app, id, "stable-id");
    assert_eq!(
        comparisons
            .iter()
            .map(|item| item.current.path.as_str())
            .collect::<Vec<_>>(),
        ["main.go", "nested/new.go"]
    );
    assert_eq!(
        comparisons[0].current.contents,
        WorkspaceFileContents::Text { text: draft.into() }
    );
    assert_eq!(
        comparisons[1].current.contents,
        WorkspaceFileContents::Missing
    );
    assert_eq!(
        fs::read_to_string(workspace.join("main.go")).unwrap(),
        draft
    );
    assert!(matches!(
        apply(&app, id, &comparisons[1]),
        AppResponse::WorkspaceFile(_)
    ));
    assert_eq!(
        fs::read_to_string(workspace.join("nested/new.go")).unwrap(),
        "package nested\n"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("main.go")).unwrap(),
        draft
    );

    fs::write(workspace.join("main.go"), "// edited after review\n").unwrap();
    assert!(matches!(
        apply(&app, id, &comparisons[0]),
        AppResponse::WorkspaceFileConflict(_)
    ));
    assert_eq!(
        fs::read_to_string(workspace.join("main.go")).unwrap(),
        "// edited after review\n"
    );
}

#[test]
fn resolves_file_fences_by_chapter_in_plain_markdown() {
    let course = TempDir::new().unwrap();
    let entry = course.path().join("lesson.md");
    fs::write(
        &entry,
        r#"# Course

## Before {#before}

```go file=before.go
package before
```

## Lesson {#stable}

> ```go file="nested/my file.go"
> package learning
> ```

```file=README.md
document
```

```go path=example.go
package example
```
"#,
    )
    .unwrap();
    let (app, _temp, id) = open(
        &entry,
        OpenCourseRequest::LocalMarkdown {
            path: entry.to_str().unwrap().into(),
        },
    );
    let comparisons = preview(&app, id, "stable");
    assert_eq!(
        comparisons
            .iter()
            .map(|file| (file.current.path.as_str(), file.proposed.as_str()))
            .collect::<Vec<_>>(),
        [
            ("README.md", "document\n"),
            ("nested/my file.go", "package learning\n"),
        ]
    );
}

#[test]
fn restores_a_deleted_starter_test_file_without_touching_its_neighbors() {
    let pack = TempDir::new().unwrap();
    let root = pack.path();
    fs::write(
        root.join("codelab.json"),
        r#"{
        "schemaVersion":"1.0","courseId":"sample","title":"Sample","starterDir":"initial"
    }"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "# Sample\n\n## Lesson\n").unwrap();
    fs::create_dir_all(root.join("initial/tests")).unwrap();
    fs::write(root.join("initial/tests/hello_test.go"), "package test\n").unwrap();
    fs::write(root.join("initial/main.go"), "package main\n").unwrap();

    let (app, temp, id) = open(
        root,
        OpenCourseRequest::LocalPack {
            path: root.to_str().unwrap().into(),
        },
    );
    let workspace = temp.path().join("workspace");
    fs::remove_dir_all(workspace.join("tests")).unwrap();
    fs::write(workspace.join("main.go"), "// edited\n").unwrap();

    let AppResponse::StarterFiles(files) = app
        .execute(AppCommand::ListStarterFiles { workspace_id: id })
        .unwrap()
    else {
        panic!("expected Starter files");
    };
    assert_eq!(files, ["main.go", "tests/hello_test.go"]);
    let AppResponse::WorkspaceComparisons(files) = app
        .execute(AppCommand::PreviewStarterFile(target(
            id,
            "tests/hello_test.go",
        )))
        .unwrap()
    else {
        panic!("expected a Starter comparison");
    };
    assert_eq!(files[0].current.contents, WorkspaceFileContents::Missing);
    assert!(!workspace.join("tests").exists());
    assert!(matches!(
        apply(&app, id, &files[0]),
        AppResponse::WorkspaceFile(_)
    ));
    assert_eq!(
        fs::read_to_string(workspace.join("tests/hello_test.go")).unwrap(),
        "package test\n"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("main.go")).unwrap(),
        "// edited\n"
    );
    assert_eq!(
        app.execute(AppCommand::PreviewStarterFile(target(id, "unrelated.go")))
            .unwrap_err()
            .code,
        codelab_core::ErrorCode::WorkspaceFileNotFound
    );
}

#[cfg(unix)]
#[test]
fn refuses_symlinked_course_sources_and_workspace_targets() {
    use std::os::unix::fs::symlink;

    let pack = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let root = pack.path();
    let outside_file = outside.path().join("untouched.go");
    fs::write(&outside_file, "outside").unwrap();
    fs::write(
        root.join("codelab.json"),
        r#"{"schemaVersion":"1.0","courseId":"sample","title":"Sample"}"#,
    )
    .unwrap();
    fs::write(
        root.join("README.md"),
        "# Sample\n\n## Lesson {#stable}\n\n```go file=outside.go\npackage outside\n```\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("solutions/stable")).unwrap();
    fs::create_dir_all(root.join("starter")).unwrap();
    fs::write(root.join("starter/main_test.go"), "package test\n").unwrap();
    symlink(&outside_file, root.join("solutions/stable/main.go")).unwrap();

    let (app, temp, id) = open(
        root,
        OpenCourseRequest::LocalPack {
            path: root.to_str().unwrap().into(),
        },
    );
    let request = || AppCommand::PreviewChapterSolution {
        workspace_id: id,
        chapter_id: "stable".into(),
    };
    assert_eq!(
        app.execute(request()).unwrap_err().code,
        ErrorCode::CoursePathInvalid
    );

    fs::remove_file(root.join("solutions/stable/main.go")).unwrap();
    fs::write(root.join("solutions/stable/main.go"), "package main\n").unwrap();
    let files = preview(&app, id, "stable");
    let workspace = temp.path().join("workspace");
    symlink(&outside_file, workspace.join("main.go")).unwrap();
    assert_eq!(
        app.execute(request()).unwrap_err().code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    let write = AppCommand::ApplyWorkspaceComparison(WorkspaceFileWrite {
        target: target(id, &files[0].current.path),
        text: files[0].proposed.clone(),
        expected_revision: files[0].current.revision.clone(),
    });
    assert_eq!(
        app.execute(write).unwrap_err().code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(fs::read_to_string(&outside_file).unwrap(), "outside");

    fs::remove_file(root.join("starter/main_test.go")).unwrap();
    symlink(&outside_file, root.join("starter/main_test.go")).unwrap();
    assert_eq!(
        app.execute(AppCommand::PreviewStarterFile(target(id, "main_test.go")))
            .unwrap_err()
            .code,
        ErrorCode::CoursePathInvalid
    );

    let missing_outside = outside.path().join("dangling.go");
    fs::remove_file(root.join("solutions/stable/main.go")).unwrap();
    symlink(&missing_outside, root.join("solutions/stable/main.go")).unwrap();
    assert_eq!(
        app.execute(request()).unwrap_err().code,
        ErrorCode::CoursePathInvalid
    );
    fs::remove_file(root.join("solutions/stable/main.go")).unwrap();
    fs::write(root.join("solutions/stable/main.go"), "package main\n").unwrap();

    fs::remove_file(root.join("starter/main_test.go")).unwrap();
    symlink(&missing_outside, root.join("starter/main_test.go")).unwrap();
    assert_eq!(
        app.execute(AppCommand::PreviewStarterFile(target(id, "main_test.go")))
            .unwrap_err()
            .code,
        ErrorCode::CoursePathInvalid
    );
    fs::remove_file(root.join("starter/main_test.go")).unwrap();
    fs::write(root.join("starter/main_test.go"), "package test\n").unwrap();

    // Dangling and internal symlinks inside the Workspace must never be compared or overwritten.
    fs::remove_file(workspace.join("main.go")).unwrap();
    symlink(&missing_outside, workspace.join("main.go")).unwrap();
    assert_eq!(
        app.execute(request()).unwrap_err().code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(
        app.execute(AppCommand::ApplyWorkspaceComparison(WorkspaceFileWrite {
            target: target(id, "main.go"),
            text: "package main\n".into(),
            expected_revision: None,
        }))
        .unwrap_err()
        .code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert!(!missing_outside.exists());

    fs::write(workspace.join("other.go"), "package other\n").unwrap();
    fs::remove_file(workspace.join("main.go")).unwrap();
    symlink("other.go", workspace.join("main.go")).unwrap();
    fs::remove_file(workspace.join("main_test.go")).unwrap();
    symlink("other.go", workspace.join("main_test.go")).unwrap();
    assert_eq!(
        app.execute(request()).unwrap_err().code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(
        app.execute(AppCommand::PreviewStarterFile(target(id, "main_test.go")))
            .unwrap_err()
            .code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(
        app.execute(AppCommand::ApplyWorkspaceComparison(WorkspaceFileWrite {
            target: target(id, "main.go"),
            text: "package main\n".into(),
            expected_revision: files[0].current.revision.clone(),
        }))
        .unwrap_err()
        .code,
        ErrorCode::WorkspacePathOutsideRoot
    );
    assert_eq!(
        fs::read_to_string(workspace.join("other.go")).unwrap(),
        "package other\n"
    );
}

#[test]
fn rejects_binary_control_byte_and_oversized_comparison_files_while_allowing_exact_limit() {
    let pack = TempDir::new().unwrap();
    let root = pack.path();
    fs::write(
        root.join("codelab.json"),
        r#"{
        "schemaVersion":"1.0","courseId":"sample","title":"Sample",
        "chapters":[{"chapterId":"stable","match":{"anchor":"stable"}}]
    }"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "# Sample\n\n## Lesson {#stable}\n").unwrap();
    fs::create_dir_all(root.join("starter")).unwrap();
    fs::create_dir_all(root.join("solutions/stable")).unwrap();
    let exact_text = "a".repeat(MAX_WORKSPACE_TEXT_BYTES);
    fs::write(root.join("starter/exact.txt"), &exact_text).unwrap();
    fs::write(root.join("starter/binary.txt"), b"bad\x01byte").unwrap();
    fs::write(
        root.join("starter/large.txt"),
        vec![b'b'; MAX_WORKSPACE_TEXT_BYTES + 1],
    )
    .unwrap();
    fs::write(root.join("starter/main.go"), "package starter\n").unwrap();
    fs::write(root.join("solutions/stable/main.go"), &exact_text).unwrap();

    let (app, temp, id) = open(
        root,
        OpenCourseRequest::LocalPack {
            path: root.to_str().unwrap().into(),
        },
    );
    let workspace = temp.path().join("workspace");

    // Exact MAX_WORKSPACE_TEXT_BYTES succeeds for both Starter and Reference Solution snapshots.
    let AppResponse::WorkspaceComparisons(starter_exact) = app
        .execute(AppCommand::PreviewStarterFile(target(id, "exact.txt")))
        .unwrap()
    else {
        panic!("expected Starter comparison");
    };
    assert_eq!(starter_exact[0].proposed.len(), MAX_WORKSPACE_TEXT_BYTES);
    let solution_exact = preview(&app, id, "stable");
    assert_eq!(solution_exact[0].proposed.len(), MAX_WORKSPACE_TEXT_BYTES);

    // Replace the seeded Workspace copies with valid text so Starter snapshot
    // validation is exercised independently of Workspace target validation.
    for path in ["binary.txt", "large.txt"] {
        fs::write(workspace.join(path), "valid learner text\n").unwrap();
        assert_eq!(
            app.execute(AppCommand::PreviewStarterFile(target(id, path)))
                .unwrap_err()
                .code,
            ErrorCode::WorkspaceFileNotText,
            "{path}"
        );
    }

    // Binary, control-byte, and oversized Reference Solution snapshots are rejected as non-text.
    for bad in [
        b"nul\0byte".as_slice(),
        b"ctrl\x07byte".as_slice(),
        [0xff, 0xfe].as_slice(),
    ] {
        fs::write(root.join("solutions/stable/main.go"), bad).unwrap();
        assert_eq!(
            app.execute(AppCommand::PreviewChapterSolution {
                workspace_id: id,
                chapter_id: "stable".into(),
            })
            .unwrap_err()
            .code,
            ErrorCode::WorkspaceFileNotText
        );
    }
    fs::write(
        root.join("solutions/stable/main.go"),
        vec![b'c'; MAX_WORKSPACE_TEXT_BYTES + 1],
    )
    .unwrap();
    assert_eq!(
        app.execute(AppCommand::PreviewChapterSolution {
            workspace_id: id,
            chapter_id: "stable".into(),
        })
        .unwrap_err()
        .code,
        ErrorCode::WorkspaceFileNotText
    );
    fs::write(root.join("solutions/stable/main.go"), "package main\n").unwrap();

    // Non-text and oversized learner Workspace targets cannot be compared against Starter or Reference Solution.
    for payload in [
        b"NUL\0data".to_vec(),
        vec![b'd'; MAX_WORKSPACE_TEXT_BYTES + 1],
    ] {
        fs::write(workspace.join("main.go"), payload).unwrap();
        assert_eq!(
            app.execute(AppCommand::PreviewChapterSolution {
                workspace_id: id,
                chapter_id: "stable".into(),
            })
            .unwrap_err()
            .code,
            ErrorCode::WorkspaceFileNotText
        );
        assert_eq!(
            app.execute(AppCommand::PreviewStarterFile(target(id, "main.go")))
                .unwrap_err()
                .code,
            ErrorCode::WorkspaceFileNotText
        );
    }
}

#[test]
fn ships_applicable_reference_files_for_all_builtin_go_exercises() {
    let trust = TempDir::new().unwrap();
    let (app, temp, id) = open(
        trust.path(),
        OpenCourseRequest::Builtin {
            course_id: "go-goframe-practice".into(),
        },
    );
    for (exercise, expected_file, expected_text) in [
        (0, "REQUIREMENTS.md", "PATCH"),
        (1, "task-api/main.go", "/healthz"),
        (2, "task-api/main.go", "sync.RWMutex"),
        (3, "task-api/internal/task/service.go", "RuneCountInString"),
        (4, "task-api/internal/task/sqlite.go", "ORDER BY id"),
        (5, "task-api/internal/task/sqlite.go", "BeginTx"),
        (6, "task-api/main.go", ".Shutdown("),
        (7, "task-api-gf/main.go", "BindHandler"),
        (8, "task-api-gf/internal/task/orm.go", ".Transaction("),
        (9, "task-api-gf/README.md", "go test -race"),
    ] {
        let comparisons = preview(&app, id, &format!("ex-{exercise}"));
        assert!(
            comparisons
                .iter()
                .any(|file| file.current.path == expected_file
                    && file.proposed.contains(expected_text)),
            "missing chapter {exercise} showcase"
        );
        for file in comparisons {
            assert!(
                matches!(apply(&app, id, &file), AppResponse::WorkspaceFile(_)),
                "applying {}",
                file.current.path
            );
            assert_eq!(
                fs::read_to_string(temp.path().join("workspace").join(&file.current.path)).unwrap(),
                file.proposed
            );
        }
    }
}

#[test]
fn ships_applicable_reference_files_and_transparent_starter_tests_for_ts_python_and_java() {
    let trust = TempDir::new().unwrap();
    for (
        course_id,
        exercise_count,
        expected_test_file,
        sample_chapter,
        sample_file,
        expected_text,
    ) in [
        (
            "ts-data-pipeline",
            8,
            "tests/ex1-filter.test.ts",
            "ex-1",
            "src/filter.ts",
            "user.age >= 18",
        ),
        (
            "python-data-clean",
            6,
            "tests/test_ex1_cleaner.py",
            "ex-1",
            "src/cleaner.py",
            "ROUND_HALF_UP",
        ),
        (
            "java-rate-limiter",
            7,
            "src/test/java/com/codelab/ratelimiter/Ex1TokenBucketTest.java",
            "ex-1",
            "src/main/java/com/codelab/ratelimiter/TokenBucket.java",
            "synchronized boolean tryAcquire",
        ),
    ] {
        let (app, temp, id) = open(
            trust.path(),
            OpenCourseRequest::Builtin {
                course_id: course_id.into(),
            },
        );
        let AppResponse::StarterFiles(starter_files) = app
            .execute(AppCommand::ListStarterFiles { workspace_id: id })
            .unwrap()
        else {
            panic!("expected starter files");
        };
        assert!(
            starter_files.iter().any(|path| path == expected_test_file),
            "{course_id} starter must include transparent test {expected_test_file}"
        );
        assert!(
            temp.path()
                .join("workspace")
                .join(expected_test_file)
                .is_file(),
            "{course_id} workspace must contain {expected_test_file}"
        );

        for index in 1..=exercise_count {
            let chapter_id = format!("ex-{index}");
            let comparisons = preview(&app, id, &chapter_id);
            assert!(
                !comparisons.is_empty(),
                "{course_id} {chapter_id} missing Reference Solution"
            );
            if chapter_id == sample_chapter {
                assert!(
                    comparisons.iter().any(|file| {
                        file.current.path == sample_file && file.proposed.contains(expected_text)
                    }),
                    "{course_id} {chapter_id} missing expected snippet"
                );
            }
            for file in comparisons {
                assert!(matches!(
                    apply(&app, id, &file),
                    AppResponse::WorkspaceFile(_)
                ));
            }
        }
    }
}
