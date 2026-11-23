#[path = "u2_course_pack_verification/git_import.rs"]
mod git_import;

use std::fs;
use std::path::{Path, PathBuf};

use codelab_studio_lib::course::{
    hydrate_starter_into_workspace, load_local_pack_dir, preview_author_draft, save_author_pack,
};
use codelab_studio_lib::domain::course::{
    ChapterKind, ChecksSpec, CourseSourceInput, MarkdownNode, OpenWorkspaceReq,
};
use codelab_studio_lib::domain::error::ErrorCode;
use codelab_studio_lib::domain::ids::{ChapterId, CommandBlockId, RelWorkspacePath, SessionEpoch};
use codelab_studio_lib::domain::runtime::Language;
use codelab_studio_lib::domain::workspace::Locale;
use codelab_studio_lib::platform::CodelabHomePaths;
use codelab_studio_lib::session::SessionOpenParams;
use codelab_studio_lib::session::{AppState, SessionLifecycleKind};
use codelab_studio_lib::storage::WorkspaceInitSpec;

fn repo_builtin_courses_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root parent")
        .join("builtin-courses")
}

fn isolated_state_with_builtins() -> (tempfile::TempDir, AppState) {
    let home = tempfile::tempdir().expect("temp home");
    let state = AppState::with_home_paths(CodelabHomePaths::from_root(home.path().to_path_buf()));
    state.set_builtin_courses_dir(Some(repo_builtin_courses_dir()));
    (home, state)
}

fn contains_markdown_node<F>(nodes: &[MarkdownNode], predicate: &F) -> bool
where
    F: Fn(&MarkdownNode) -> bool,
{
    for node in nodes {
        if predicate(node) {
            return true;
        }
        let nested = match node {
            MarkdownNode::Paragraph { children }
            | MarkdownNode::Heading { children, .. }
            | MarkdownNode::Strong { children }
            | MarkdownNode::Emphasis { children }
            | MarkdownNode::BlockQuote { children }
            | MarkdownNode::Link { children, .. }
            | MarkdownNode::Superscript { children }
            | MarkdownNode::Subscript { children }
            | MarkdownNode::Details { children, .. } => contains_markdown_node(children, predicate),
            MarkdownNode::List { items, .. } => items
                .iter()
                .any(|item| contains_markdown_node(item, predicate)),
            MarkdownNode::Table { headers, rows } => {
                headers
                    .iter()
                    .any(|cell| contains_markdown_node(cell, predicate))
                    || rows.iter().any(|row| {
                        row.iter()
                            .any(|cell| contains_markdown_node(cell, predicate))
                    })
            }
            MarkdownNode::Text { .. }
            | MarkdownNode::InlineCode { .. }
            | MarkdownNode::CodeBlock { .. }
            | MarkdownNode::Image { .. }
            | MarkdownNode::LineBreak
            | MarkdownNode::ThematicBreak
            | MarkdownNode::Kbd { .. } => false,
        };
        if nested {
            return true;
        }
    }
    false
}

fn write_sample_pack(dir: &Path, title: &str, starter_note: &str) {
    fs::create_dir_all(dir.join("starter")).expect("create starter dir");
    fs::write(
        dir.join("codelab.json"),
        format!(
            r#"{{
  "schemaVersion": "1.0",
  "courseId": "sample-ext-course",
  "title": "{title}",
  "description": "External course pack for verification",
  "language": "go",
  "entryFile": "README.md",
  "starterDir": "starter",
  "checksFile": ".codelab-checks.json",
  "chapters": [
    {{
      "chapterId": "ex-1",
      "title": "Exercise 1",
      "kind": "exercise",
      "match": {{ "headingPrefix": "Exercise 1" }},
      "defaultCwd": "app"
    }}
  ]
}}"#
        ),
    )
    .expect("write codelab.json");

    fs::write(
        dir.join("README.md"),
        format!(
            "# {title}\n\n\
             ## Overview\n\n\
             Reading introduction.\n\n\
             ## Exercise 1: Build Health Endpoint\n\n\
             Implement a minimal HTTP endpoint.\n\n\
             ```bash\n\
             go test -race ./...\n\
             ```\n"
        ),
    )
    .expect("write README.md");

    fs::write(
        dir.join(".codelab-checks.json"),
        r#"{
  "schemaVersion": "1.0",
  "courseId": "sample-ext-course",
  "language": "go",
  "chapters": [
    {
      "chapterId": "ex-1",
      "checks": [
        {
          "id": "v1",
          "title": "Health endpoint responds 200",
          "type": "http_assert",
          "request": { "method": "GET", "url": "http://127.0.0.1:8080/healthz" },
          "expect": { "status": 200 }
        }
      ]
    }
  ]
}"#,
    )
    .expect("write .codelab-checks.json");

    fs::write(dir.join("starter").join("hello.txt"), starter_note).expect("write starter file");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn builtin_go_pack_loads_via_bootstrap_and_binds_all_ten_exercises_with_cwd_overlay() {
    let (_home, state) = isolated_state_with_builtins();

    let bootstrap = state
        .build_bootstrap_state(Locale::ZhCn)
        .expect("bootstrap with built-in courses");
    assert_eq!(bootstrap.builtin_courses.len(), 1);
    let summary = &bootstrap.builtin_courses[0];
    assert_eq!(summary.course_id, "go-goframe-practice");
    assert_eq!(summary.chapter_count, 10);
    assert_eq!(summary.language, Language::Go);
    assert_eq!(summary.entry_file.as_str(), "README.md");
    assert!(!summary.description.is_empty());

    let inspect = state
        .inspect_course_source(CourseSourceInput::Builtin {
            course_id: "go-goframe-practice".to_owned(),
        })
        .await
        .expect("inspect built-in go pack");

    assert!(!inspect.requires_trust);
    assert!(inspect.trusted);
    assert!(
        inspect.validation_issues.is_empty(),
        "unexpected validation issues: {:?}",
        inspect.validation_issues
    );
    assert_eq!(inspect.course.chapters.len(), 15);

    let exercises: Vec<_> = inspect
        .course
        .chapters
        .iter()
        .filter(|ch| ch.kind == ChapterKind::Exercise)
        .collect();
    let readings: Vec<_> = inspect
        .course
        .chapters
        .iter()
        .filter(|ch| ch.kind == ChapterKind::Reading)
        .collect();
    assert_eq!(exercises.len(), 10);
    assert_eq!(readings.len(), 5);

    let expected_ids: Vec<ChapterId> = (0..10).map(|i| ChapterId(format!("ex-{i}"))).collect();
    let actual_ids: Vec<ChapterId> = exercises.iter().map(|ch| ch.chapter_id.clone()).collect();
    assert_eq!(actual_ids, expected_ids);

    let ex3 = exercises
        .iter()
        .find(|ch| ch.chapter_id.0 == "ex-3")
        .expect("ex-3 present");
    assert_eq!(
        ex3.default_cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("task-api")
    );
    let ex3_test_cmd = ex3
        .command_blocks
        .iter()
        .find(|cmd| cmd.command.contains("go test"))
        .expect("ex-3 go test command");
    assert_eq!(
        ex3_test_cmd.cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("task-api")
    );
    assert!(ex3_test_cmd.id.0.starts_with("ex-3-cmd-"));
    assert!(contains_markdown_node(&ex3.nodes, &|node| matches!(
        node,
        MarkdownNode::CodeBlock {
            command_block_id: Some(id),
            ..
        } if *id == ex3_test_cmd.id
    )));

    let ex4 = exercises
        .iter()
        .find(|ch| ch.chapter_id.0 == "ex-4")
        .expect("ex-4 present");
    let ex4_get_cmd = ex4
        .command_blocks
        .iter()
        .find(|cmd| cmd.command.contains("go get"))
        .expect("ex-4 go get command");
    assert_eq!(
        ex4_get_cmd.cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("task-api")
    );
    let sql_ref = ex4
        .three_step
        .step2_reference_blocks
        .iter()
        .find(|blk| {
            blk.target_path
                .as_ref()
                .is_some_and(|p| p.as_str().ends_with("migrations/001_create_tasks.sql"))
        })
        .expect("ex-4 sql reference block");
    assert_eq!(
        sql_ref.target_path.as_ref().map(RelWorkspacePath::as_str),
        Some("task-api/migrations/001_create_tasks.sql")
    );
    assert!(contains_markdown_node(&ex4.nodes, &|node| matches!(
        node,
        MarkdownNode::CodeBlock {
            target_path: Some(path),
            command_block_id: None,
            ..
        } if path.as_str() == "task-api/migrations/001_create_tasks.sql"
    )));

    let ex7 = exercises
        .iter()
        .find(|ch| ch.chapter_id.0 == "ex-7")
        .expect("ex-7 present");
    let ex7_gf_cmd = ex7
        .command_blocks
        .iter()
        .find(|cmd| cmd.command.contains("gf init"))
        .expect("ex-7 gf init command");
    assert_eq!(ex7_gf_cmd.cwd, None);

    match inspect.checks {
        Some(ChecksSpec::MultiChapter(multi)) => assert_eq!(multi.chapters.len(), 10),
        other => panic!("expected MultiChapter ChecksSpec with 10 chapters, got {other:?}"),
    }
    assert!(inspect
        .starter_files
        .contains(&RelWorkspacePath::new("LEARNING_NOTES.md").unwrap()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn starter_hydration_is_idempotent_and_preserves_learner_edits_and_deletions() {
    let (_home, state) = isolated_state_with_builtins();
    let ws_parent = tempfile::tempdir().expect("ws parent");
    let ws1 = ws_parent.path().join("fresh-ws");
    let notes_rel = RelWorkspacePath::new("LEARNING_NOTES.md").unwrap();

    let open1 = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: CourseSourceInput::Builtin {
                course_id: "go-goframe-practice".to_owned(),
            },
            workspace_root: ws1.to_string_lossy().into_owned(),
            create_if_missing: true,
            accept_trust: false,
            expected_fingerprint: None,
        })
        .await
        .expect("first open hydrates starter");

    assert_eq!(open1.newly_hydrated_files, vec![notes_rel.clone()]);
    assert!(open1.skipped_existing_files.is_empty());
    assert!(open1.workspace_meta.hydrated_files.contains(&notes_rel));
    let notes_abs = ws1.join("LEARNING_NOTES.md");
    assert!(notes_abs.is_file());

    fs::write(&notes_abs, "learner edited notes\n").expect("learner edits starter file");
    state
        .close_workspace(open1.epoch)
        .await
        .expect("close workspace after edit");

    let open2 = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: CourseSourceInput::Builtin {
                course_id: "go-goframe-practice".to_owned(),
            },
            workspace_root: ws1.to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: false,
            expected_fingerprint: None,
        })
        .await
        .expect("reopen preserves learner edits");

    assert!(open2.newly_hydrated_files.is_empty());
    assert!(open2.skipped_existing_files.is_empty());
    assert_eq!(
        fs::read_to_string(&notes_abs).expect("read edited file"),
        "learner edited notes\n"
    );

    fs::remove_file(&notes_abs).expect("learner deletes starter file");
    state
        .close_workspace(open2.epoch)
        .await
        .expect("close workspace after deletion");

    let open3 = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: CourseSourceInput::Builtin {
                course_id: "go-goframe-practice".to_owned(),
            },
            workspace_root: ws1.to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: false,
            expected_fingerprint: None,
        })
        .await
        .expect("reopen after deletion");

    assert!(open3.newly_hydrated_files.is_empty());
    assert!(
        !notes_abs.exists(),
        "deleted starter file must never be resurrected"
    );
    state.close_workspace(open3.epoch).await.expect("close ws1");

    let ws2 = tempfile::tempdir().expect("preexisting ws");
    let pre_existing_notes = ws2.path().join("LEARNING_NOTES.md");
    fs::write(&pre_existing_notes, "existing personal journal\n").expect("write preexisting file");

    let open4 = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: CourseSourceInput::Builtin {
                course_id: "go-goframe-practice".to_owned(),
            },
            workspace_root: ws2.path().to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: false,
            expected_fingerprint: None,
        })
        .await
        .expect("open existing workspace without overwriting starter");

    assert!(open4.newly_hydrated_files.is_empty());
    assert_eq!(open4.skipped_existing_files, vec![notes_rel.clone()]);
    assert!(open4.workspace_meta.hydrated_files.contains(&notes_rel));
    assert_eq!(
        fs::read_to_string(&pre_existing_notes).expect("read untouched preexisting file"),
        "existing personal journal\n"
    );

    state.close_workspace(open4.epoch).await.expect("close ws2");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn starter_hydration_rejects_files_changed_after_pack_inspection() {
    let (_home, state) = isolated_state_with_builtins();
    let pack_dir = tempfile::tempdir().expect("pack dir");
    write_sample_pack(pack_dir.path(), "Fingerprint Demo", "trusted starter\n");

    let pack = load_local_pack_dir(pack_dir.path()).expect("load external pack");
    let starter_dir = pack.starter_dir_abs.clone().expect("starter directory");
    let expected_fingerprints = pack.starter_fingerprints.clone();
    let workspace = tempfile::tempdir().expect("workspace");
    let params = SessionOpenParams::new(
        workspace.path().to_path_buf(),
        false,
        WorkspaceInitSpec {
            course_id: "sample-ext-course".to_owned(),
            course_source: pack.source.clone(),
            pack_fingerprint: pack.pack_fingerprint.clone(),
        },
        pack.course.title.clone(),
        Language::Go,
        pack.course.chapters.len() as u32,
    );
    let session = state.open_workspace(params).await.expect("open workspace");

    fs::write(starter_dir.join("hello.txt"), "changed after inspection\n")
        .expect("tamper starter after fingerprinting");
    let error = hydrate_starter_into_workspace(
        &starter_dir,
        session.paths().root(),
        session.store(),
        &expected_fingerprints,
    )
    .expect_err("changed starter must fail fingerprint verification");

    assert_eq!(error.code, ErrorCode::ExternalPackUntrusted);
    assert!(!workspace.path().join("hello.txt").exists());

    fs::write(starter_dir.join("hello.txt"), "trusted starter\n")
        .expect("restore inspected starter content");
    fs::write(
        starter_dir.join("added-after-inspection.txt"),
        "untrusted\n",
    )
    .expect("add uninspected starter file");
    let error = hydrate_starter_into_workspace(
        &starter_dir,
        session.paths().root(),
        session.store(),
        &expected_fingerprints,
    )
    .expect_err("added starter file must fail fingerprint verification");
    assert_eq!(error.code, ErrorCode::ExternalPackUntrusted);
    assert!(!workspace.path().join("hello.txt").exists());
    assert!(!workspace.path().join("added-after-inspection.txt").exists());

    state
        .close_workspace(session.epoch())
        .await
        .expect("close workspace");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_pack_enforces_trust_gate_toctou_fingerprint_schema_and_symlink_guards() {
    let (_home, state) = isolated_state_with_builtins();
    let pack_dir = tempfile::tempdir().expect("external pack dir");
    write_sample_pack(pack_dir.path(), "External Trust Demo", "starter v1\n");
    let ws = tempfile::tempdir().expect("workspace dir");

    let source = CourseSourceInput::LocalPackDir {
        dir_path: pack_dir.path().to_string_lossy().into_owned(),
    };
    let inspect1 = state
        .inspect_course_source(source.clone())
        .await
        .expect("inspect external pack");
    assert!(inspect1.requires_trust);
    assert!(!inspect1.trusted);
    assert!(inspect1.validation_issues.is_empty());
    assert!(matches!(
        state.lifecycle_state(),
        SessionLifecycleKind::TrustPending { .. }
    ));

    let untrusted_err = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: source.clone(),
            workspace_root: ws.path().to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: false,
            expected_fingerprint: None,
        })
        .await
        .expect_err("untrusted external pack must be blocked");
    assert_eq!(untrusted_err.code, ErrorCode::ExternalPackUntrusted);
    assert!(!ws.path().join(".codelab").exists());
    assert!(!ws.path().join("hello.txt").exists());

    let toctou_err = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: source.clone(),
            workspace_root: ws.path().to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: true,
            expected_fingerprint: Some("sha256:stale-fingerprint".to_owned()),
        })
        .await
        .expect_err("mismatched fingerprint must be rejected");
    assert_eq!(toctou_err.code, ErrorCode::ExternalPackUntrusted);
    assert!(!ws.path().join(".codelab").exists());

    let opened = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: source.clone(),
            workspace_root: ws.path().to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: true,
            expected_fingerprint: Some(inspect1.pack_fingerprint.clone()),
        })
        .await
        .expect("matching fingerprint trusts and opens");
    assert_eq!(
        fs::read_to_string(ws.path().join("hello.txt")).expect("read hydrated hello.txt"),
        "starter v1\n"
    );
    state
        .close_workspace(opened.epoch)
        .await
        .expect("close trusted workspace");

    let reopened = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: source.clone(),
            workspace_root: ws.path().to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: false,
            expected_fingerprint: None,
        })
        .await
        .expect("previously trusted pack reopens without re-prompt");
    state
        .close_workspace(reopened.epoch)
        .await
        .expect("close reopened workspace");

    fs::write(
        pack_dir.path().join("starter").join("hello.txt"),
        "starter v2 tampered\n",
    )
    .expect("modify starter file");
    let inspect2 = state
        .inspect_course_source(source.clone())
        .await
        .expect("re-inspect modified pack");
    assert_ne!(inspect2.pack_fingerprint, inspect1.pack_fingerprint);
    assert!(!inspect2.trusted);

    let invalidated_err = state
        .trust_and_open_course(OpenWorkspaceReq {
            source,
            workspace_root: ws.path().to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: false,
            expected_fingerprint: None,
        })
        .await
        .expect_err("modified pack invalidates prior trust");
    assert_eq!(invalidated_err.code, ErrorCode::ExternalPackUntrusted);

    let bad_manifest_dir = tempfile::tempdir().expect("bad manifest dir");
    fs::write(
        bad_manifest_dir.path().join("codelab.json"),
        "{not-valid-json",
    )
    .unwrap();
    fs::write(bad_manifest_dir.path().join("README.md"), "# Bad\n").unwrap();
    let manifest_err = state
        .inspect_course_source(CourseSourceInput::LocalPackDir {
            dir_path: bad_manifest_dir.path().to_string_lossy().into_owned(),
        })
        .await
        .expect_err("malformed codelab.json must fail");
    assert_eq!(manifest_err.code, ErrorCode::SchemaValidationFailed);

    let bad_checks_dir = tempfile::tempdir().expect("bad checks dir");
    fs::write(
        bad_checks_dir.path().join("README.md"),
        "# Bad Checks\n\n## Ex 1\n\nHi.\n",
    )
    .unwrap();
    fs::write(
        bad_checks_dir.path().join(".codelab-checks.json"),
        r#"{"schemaVersion":"2.0","chapters":[]}"#,
    )
    .unwrap();
    let checks_err = state
        .inspect_course_source(CourseSourceInput::LocalPackDir {
            dir_path: bad_checks_dir.path().to_string_lossy().into_owned(),
        })
        .await
        .expect_err("invalid checks schema version must fail");
    assert_eq!(checks_err.code, ErrorCode::SchemaValidationFailed);

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let outside_dir = tempfile::tempdir().expect("outside secret dir");
        let secret_file = outside_dir.path().join("secret.txt");
        fs::write(&secret_file, "top secret").unwrap();

        let symlink_pack = tempfile::tempdir().expect("symlink pack");
        write_sample_pack(symlink_pack.path(), "Symlink Attack Pack", "ok\n");
        symlink(
            &secret_file,
            symlink_pack.path().join("starter").join("escape.txt"),
        )
        .expect("create symlink inside starter");

        let sym_err = state
            .inspect_course_source(CourseSourceInput::LocalPackDir {
                dir_path: symlink_pack.path().to_string_lossy().into_owned(),
            })
            .await
            .expect_err("symlink inside starter must be rejected");
        assert_eq!(sym_err.code, ErrorCode::PathOutsideSandbox);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn comment_anchor_and_reload_manual_respect_session_epoch() {
    let (_home, state) = isolated_state_with_builtins();
    let pack_dir = tempfile::tempdir().expect("anchor pack dir");
    let ws_dir = tempfile::tempdir().expect("anchor ws dir");

    fs::write(
        pack_dir.path().join("codelab.json"),
        r#"{
  "schemaVersion": "1.0",
  "courseId": "anchor-course",
  "title": "Anchor Course",
  "description": "Comment anchor durability check",
  "language": "go",
  "entryFile": "README.md",
  "chapters": [
    {
      "chapterId": "ex-1",
      "title": "Original Title",
      "kind": "exercise",
      "defaultCwd": "svc"
    }
  ]
}"#,
    )
    .expect("write codelab.json");

    fs::write(
        pack_dir.path().join("README.md"),
        "# Anchor Course\n\n\
         ## Preface\n\n\
         Background reading before exercise.\n\n\
         <!-- codelab:chapter id=\"ex-1\" -->\n\
         ## Completely Renamed Heading\n\n\
         First goal sentence.\n\n\
         ```bash\n\
         go test ./...\n\
         ```\n",
    )
    .expect("write initial README.md");

    let opened = state
        .trust_and_open_course(OpenWorkspaceReq {
            source: CourseSourceInput::LocalPackDir {
                dir_path: pack_dir.path().to_string_lossy().into_owned(),
            },
            workspace_root: ws_dir.path().to_string_lossy().into_owned(),
            create_if_missing: false,
            accept_trust: true,
            expected_fingerprint: None,
        })
        .await
        .expect("open anchor course");

    assert_eq!(opened.course.chapters.len(), 2);
    assert_eq!(opened.course.chapters[0].kind, ChapterKind::Reading);
    assert_eq!(
        opened.course.chapters[1].chapter_id,
        ChapterId("ex-1".to_owned())
    );
    assert_eq!(opened.course.chapters[1].kind, ChapterKind::Exercise);
    assert_eq!(
        opened.course.chapters[1].command_blocks[0].id,
        CommandBlockId("ex-1-cmd-01".to_owned())
    );
    assert_eq!(
        opened.course.chapters[1].command_blocks[0]
            .cwd
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("svc")
    );

    fs::write(
        pack_dir.path().join("README.md"),
        "# Anchor Course\n\n\
         ## Newly Inserted Intro\n\n\
         Extra reading section.\n\n\
         ## Preface\n\n\
         Background reading before exercise.\n\n\
         <!-- codelab:chapter id=\"ex-1\" -->\n\
         ## Renamed Again After Edit\n\n\
         Updated goal sentence after hot reload.\n\n\
         ```bash\n\
         go test -race ./...\n\
         ```\n",
    )
    .expect("update README.md");

    let stale_err = state
        .reload_active_manual(SessionEpoch(opened.epoch.0 + 99))
        .await
        .expect_err("stale session epoch must be rejected");
    assert_eq!(stale_err.code, ErrorCode::SessionEpochStale);

    let reloaded = state
        .reload_active_manual(opened.epoch)
        .await
        .expect("live epoch reloads manual");
    assert_eq!(reloaded.chapters.len(), 3);
    assert_eq!(reloaded.chapters[0].kind, ChapterKind::Reading);
    assert_eq!(reloaded.chapters[1].kind, ChapterKind::Reading);
    assert_eq!(
        reloaded.chapters[2].chapter_id,
        ChapterId("ex-1".to_owned())
    );
    assert_eq!(reloaded.chapters[2].kind, ChapterKind::Exercise);
    assert_eq!(
        reloaded.chapters[2].goal.as_deref(),
        Some("Updated goal sentence after hot reload.")
    );
    assert!(reloaded.chapters[2].command_blocks[0]
        .command
        .contains("-race"));

    state
        .close_workspace(opened.epoch)
        .await
        .expect("close anchor workspace");
}

#[test]
fn author_preview_and_atomic_save_validate_checks_before_writing() {
    let draft_md = "# Authored Course\n\n\
                    ## Exercise 1 {#ex-1}\n\n\
                    Build the health endpoint.\n\n\
                    ```bash\n\
                    go run .\n\
                    ```\n";

    let bad_json_preview = preview_author_draft(draft_md, Some("{broken-json"))
        .expect("preview returns issues instead of failing hard");
    assert!(bad_json_preview.checks.is_none());
    assert!(!bad_json_preview.validation_issues.is_empty());
    assert_eq!(bad_json_preview.course.title, "Authored Course");

    let ssrf_and_missing_ch = r#"{
  "schemaVersion": "1.0",
  "chapterId": "missing-ch",
  "language": "go",
  "checks": [
    {
      "id": "v1",
      "title": "Blocked metadata SSRF",
      "type": "http_assert",
      "request": { "method": "GET", "url": "http://169.254.169.254/latest/meta-data" },
      "expect": { "status": 200 }
    }
  ]
}"#;
    let semantic_preview = preview_author_draft(draft_md, Some(ssrf_and_missing_ch))
        .expect("preview parses schema and surfaces semantic issues");
    assert!(semantic_preview.checks.is_some());
    assert_eq!(semantic_preview.validation_issues.len(), 2);

    let pack_dir = tempfile::tempdir().expect("author pack dir");
    let readme_path = pack_dir.path().join("README.md");
    let checks_path = pack_dir.path().join(".codelab-checks.json");
    fs::write(&readme_path, "# Original\n").expect("write original README.md");
    fs::write(&checks_path, "original-checks").expect("write original checks");

    let malformed_err = save_author_pack(pack_dir.path(), draft_md, "{invalid")
        .expect_err("malformed checks JSON must block save");
    assert_eq!(malformed_err.code, ErrorCode::SchemaValidationFailed);
    assert_eq!(fs::read_to_string(&readme_path).unwrap(), "# Original\n");
    assert_eq!(fs::read_to_string(&checks_path).unwrap(), "original-checks");

    let ssrf_err = save_author_pack(pack_dir.path(), draft_md, ssrf_and_missing_ch)
        .expect_err("SSRF or unknown chapter checks must block save");
    assert_eq!(ssrf_err.code, ErrorCode::SchemaValidationFailed);
    assert_eq!(fs::read_to_string(&readme_path).unwrap(), "# Original\n");
    assert_eq!(fs::read_to_string(&checks_path).unwrap(), "original-checks");

    let valid_checks = r#"{
  "schemaVersion": "1.0",
  "chapterId": "ex-1",
  "language": "go",
  "checks": [
    {
      "id": "v1",
      "title": "Loopback health check",
      "type": "http_assert",
      "request": { "method": "GET", "url": "http://127.0.0.1:8080/healthz" },
      "expect": { "status": 200 }
    }
  ]
}"#;
    let saved_course =
        save_author_pack(pack_dir.path(), draft_md, valid_checks).expect("valid author save");
    assert_eq!(saved_course.title, "Authored Course");
    assert_eq!(saved_course.chapters.len(), 1);
    assert_eq!(
        saved_course.chapters[0].chapter_id,
        ChapterId("ex-1".to_owned())
    );
    assert_eq!(fs::read_to_string(&readme_path).unwrap(), draft_md);
    assert_eq!(fs::read_to_string(&checks_path).unwrap(), valid_checks);

    let leftovers: Vec<_> = fs::read_dir(pack_dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains(".tmp."))
        .collect();
    assert!(leftovers.is_empty());
}

#[test]
fn author_save_rejects_entry_and_checks_paths_that_collide() {
    let pack_dir = tempfile::tempdir().expect("author pack dir");
    write_sample_pack(pack_dir.path(), "Collision Demo", "starter\n");

    let manifest_path = pack_dir.path().join("codelab.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).expect("read manifest"))
            .expect("parse manifest");
    manifest["entryFile"] = serde_json::Value::String("shared.json".to_owned());
    manifest["checksFile"] = serde_json::Value::String("shared.json".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_vec(&manifest).expect("serialize manifest"),
    )
    .expect("write colliding manifest");

    let shared_file = pack_dir.path().join("shared.json");
    fs::write(&shared_file, "preserve original").expect("write destination");
    let error = save_author_pack(
        pack_dir.path(),
        "# Collision Demo\n\n## Exercise 1 {#ex-1}\n\nBuild it.\n",
        r#"{"schemaVersion":"1.0","chapterId":"ex-1","language":"go","checks":[]}"#,
    )
    .expect_err("author save must reject colliding destinations");

    assert_eq!(error.code, ErrorCode::SchemaValidationFailed);
    assert_eq!(
        fs::read_to_string(shared_file).unwrap(),
        "preserve original"
    );

    manifest["entryFile"] = serde_json::Value::String("README.md".to_owned());
    manifest["checksFile"] = serde_json::Value::String("codelab.json".to_owned());
    let original_manifest = serde_json::to_vec(&manifest).unwrap();
    fs::write(&manifest_path, &original_manifest).unwrap();
    let original_entry = fs::read(pack_dir.path().join("README.md")).unwrap();
    let error = save_author_pack(
        pack_dir.path(),
        "# Collision Demo\n\n## Exercise 1 {#ex-1}\n\nBuild it.\n",
        r#"{"schemaVersion":"1.0","chapterId":"ex-1","language":"go","checks":[]}"#,
    )
    .expect_err("author save must preserve the manifest");
    assert_eq!(error.code, ErrorCode::SchemaValidationFailed);
    assert_eq!(fs::read(manifest_path).unwrap(), original_manifest);
    assert_eq!(
        fs::read(pack_dir.path().join("README.md")).unwrap(),
        original_entry
    );
}
