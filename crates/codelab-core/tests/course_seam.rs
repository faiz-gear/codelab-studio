use std::fs;
use std::time::{Duration, Instant};

use codelab_core::{
    App, AppCommand, AppConfig, AppEvent, AppResponse, ChapterKind, ChapterPresentation, CheckKind,
    CommandMode, ContentBlock, CourseLanguage, CourseSource, ErrorCode, MAX_MARKDOWN_BYTES,
    OpenCourseRequest, ValueOrigin,
};
use tempfile::TempDir;

#[test]
fn builtin_go_goframe_course_pack_loads_with_exercises_0_to_9_and_three_step_projection() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");
    let events = app.subscribe();

    // Bootstrap exposes built-in course catalog
    let bootstrap = match app.execute(AppCommand::Bootstrap).expect("bootstrap") {
        AppResponse::Bootstrap(b) => b,
        other => panic!("expected Bootstrap, got {other:?}"),
    };
    let catalog = match app
        .execute(AppCommand::ListBuiltinCourses)
        .expect("list builtin courses")
    {
        AppResponse::BuiltinCourses(list) => list,
        other => panic!("expected BuiltinCourses, got {other:?}"),
    };
    assert_eq!(bootstrap.builtin_courses, catalog);
    let go_summary = catalog
        .iter()
        .find(|c| c.course_id == "go-goframe-practice")
        .expect("go-goframe-practice must be in built-in catalog");
    assert_eq!(go_summary.language, Some(CourseLanguage::Go));
    assert_eq!(go_summary.total_exercises, 10);
    assert!(go_summary.is_builtin);

    // Opening built-in Go/GoFrame Course Pack emits CourseLoaded and parses exercises 0-9 + guides
    let course = match app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::Builtin {
            course_id: "go-goframe-practice".into(),
        }))
        .expect("open builtin go course")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected LoadedCourse, got {other:?}"),
    };
    let drained: Vec<_> = std::iter::from_fn(|| events.try_recv().ok()).collect();
    let loaded_event = drained
        .iter()
        .find(|event| matches!(event, AppEvent::CourseLoaded(_)))
        .unwrap_or_else(|| panic!("expected CourseLoaded event, saw: {drained:?}"));
    assert_eq!(loaded_event, &AppEvent::CourseLoaded(course.clone()));

    assert_eq!(course.course_id, "go-goframe-practice");
    assert_eq!(course.language, Some(CourseLanguage::Go));
    assert_eq!(course.total_exercises, 10);
    assert!(course.is_builtin);

    let exercise_ids: Vec<&str> = course
        .chapters
        .iter()
        .filter(|ch| ch.kind == ChapterKind::Exercise)
        .map(|ch| ch.chapter_id.as_str())
        .collect();
    assert_eq!(
        exercise_ids,
        vec![
            "ex-0", "ex-1", "ex-2", "ex-3", "ex-4", "ex-5", "ex-6", "ex-7", "ex-8", "ex-9"
        ]
    );

    let guide_count = course
        .chapters
        .iter()
        .filter(|ch| ch.kind == ChapterKind::Guide)
        .count();
    assert!(
        guide_count >= 2,
        "built-in GoFrame handbook should include concept guide chapters"
    );

    // Inspect ex-1 three-step projection, daemon inference, folded curl command, checks, and reflections
    let ex1 = course
        .chapters
        .iter()
        .find(|ch| ch.chapter_id == "ex-1")
        .expect("ex-1 present");
    assert_eq!(ex1.kind, ChapterKind::Exercise);
    assert_eq!(ex1.presentation, ChapterPresentation::ThreeStep);
    assert_eq!(ex1.entry_file.as_deref(), Some("task-api/main.go"));

    let three_step = ex1
        .three_step
        .as_ref()
        .expect("ex-1 must have three-step projection");

    let daemon_cmd = ex1
        .commands
        .iter()
        .find(|cmd| cmd.command.contains("go run ."))
        .expect("go run . command block");
    assert_eq!(daemon_cmd.mode, CommandMode::Daemon);
    assert_eq!(daemon_cmd.cwd.as_deref(), Some("task-api"));
    assert!(
        three_step.step1_command_ids.contains(&daemon_cmd.id),
        "daemon start command must appear in Step 1"
    );

    let curl_cmd = ex1
        .commands
        .iter()
        .find(|cmd| cmd.command.contains("curl"))
        .expect("curl debug command block");
    assert_eq!(curl_cmd.mode, CommandMode::Oneshot);
    assert!(curl_cmd.is_secondary);
    assert!(
        three_step.secondary_command_ids.contains(&curl_cmd.id),
        "curl command must be folded into secondary debug commands"
    );

    assert_eq!(
        three_step.step2_entry_file.as_deref(),
        Some("task-api/main.go")
    );
    assert!(
        three_step
            .step2_html
            .as_deref()
            .unwrap_or_default()
            .contains("http.NewServeMux"),
        "Step 2 should retain main.go guidance"
    );
    assert!(
        !three_step
            .step1_intro_html
            .as_deref()
            .unwrap_or_default()
            .contains("在另一个终端执行"),
        "secondary curl lead-in must not leak into Step 1 intro"
    );
    assert!(
        !three_step
            .step2_html
            .as_deref()
            .unwrap_or_default()
            .contains("在学习工作区中建立项目："),
        "primary command lead-in must not stay orphaned in Step 2"
    );

    // Inspect ex-6: nested curl inside ordered list is extracted to secondary commands without duplicating in Step 2 prose
    let ex6 = course
        .chapters
        .iter()
        .find(|ch| ch.chapter_id == "ex-6")
        .expect("ex-6 present");
    let ex6_three_step = ex6.three_step.as_ref().expect("ex-6 three-step");
    let ex6_curl = ex6
        .commands
        .iter()
        .find(|cmd| cmd.command.contains("/debug/wait"))
        .expect("ex-6 nested curl extracted");
    assert!(ex6_curl.is_secondary);
    assert!(ex6_three_step.secondary_command_ids.contains(&ex6_curl.id));
    assert!(
        !ex6_three_step
            .step2_html
            .as_deref()
            .unwrap_or_default()
            .contains("curl --max-time 0.2"),
        "nested shell fence extracted into CommandBlock must not duplicate in Step 2 prose"
    );

    let daemon_req = three_step
        .requires_daemon
        .as_ref()
        .expect("ex-1 requiresDaemon");
    assert_eq!(daemon_req.port, 8080);
    assert_eq!(three_step.step3_checks.len(), 3);
    assert!(
        three_step
            .step3_checks
            .iter()
            .all(|check| check.kind == CheckKind::HttpAssert)
    );
    assert!(
        three_step
            .reflections_html
            .as_deref()
            .unwrap_or_default()
            .contains("http.Server"),
        "reflection prompts should be extracted for <details> folding"
    );

    // Unknown built-in course returns structured CourseNotFound error
    let err = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::Builtin {
            course_id: "does-not-exist".into(),
        }))
        .expect_err("unknown builtin course must fail");
    assert_eq!(err.code, ErrorCode::CourseNotFound);
    assert_eq!(
        err.details.get("source").map(String::as_str),
        Some("does-not-exist")
    );
}

#[test]
fn deterministic_chapter_id_priority_unicode_nfkc_dedup_and_guide_vs_exercise_classification() {
    let dir = TempDir::new().expect("temp dir");
    let pack_dir = dir.path().join("priority-pack");
    fs::create_dir_all(&pack_dir).expect("create pack dir");

    fs::write(
        pack_dir.join("codelab.json"),
        r#"{
  "schemaVersion": "1.0",
  "courseId": "priority-demo",
  "title": "Priority Demo",
  "language": "go",
  "entryFile": "README.md",
  "chapters": [
    {
      "chapterId": "manifest-wins",
      "title": "练习 1：清单映射优先",
      "kind": "exercise",
      "match": { "headingPrefix": "练习 1：" }
    },
    {
      "chapterId": "forced-guide",
      "kind": "guide",
      "match": { "anchor": "guide-anchor" }
    }
  ]
}"#,
    )
    .expect("write manifest");

    fs::write(
        pack_dir.join("README.md"),
        r#"# Priority Demo

## 练习 1：清单映射优先 {#ignored-anchor}
Manifest mapping beats both H2 tail anchor and numbered pattern.

## 练习 2：锚点优先 {#anchor-beats-pattern}
H2 tail anchor beats numbered pattern `ex-2`.

## 概念导读{#no-space-anchor}
Guide without space before tail anchor.

## 练习 3：中文编号模式
Exercise 3 resolved via numbered pattern.

## Exercise 04: English Pattern
Exercise 4 resolved via English pattern with leading zero normalized.

## 练习 99：清单标为导读 {#guide-anchor}
Explicit manifest `kind: "guide"` overrides numbered exercise pattern.

## ＡＰＩ　设计
Fullwidth Unicode heading.

## API 设计
ASCII + CJK heading colliding after NFKC normalization.

## Setup & Config
First duplicate slug.

## Setup & Config
Second duplicate slug.
"#,
    )
    .expect("write markdown");

    let app = App::open(AppConfig {
        data_dir: dir.path().join("app-data"),
    })
    .expect("open app");

    let course = match app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
            path: pack_dir.to_string_lossy().into_owned(),
        }))
        .expect("open local pack")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected LoadedCourse, got {other:?}"),
    };

    let summary: Vec<(&str, &str, ChapterKind)> = course
        .chapters
        .iter()
        .map(|ch| (ch.chapter_id.as_str(), ch.title.as_str(), ch.kind))
        .collect();

    assert_eq!(
        summary,
        vec![
            (
                "manifest-wins",
                "练习 1：清单映射优先",
                ChapterKind::Exercise
            ),
            (
                "anchor-beats-pattern",
                "练习 2：锚点优先",
                ChapterKind::Exercise
            ),
            ("no-space-anchor", "概念导读", ChapterKind::Guide),
            ("ex-3", "练习 3：中文编号模式", ChapterKind::Exercise),
            (
                "ex-4",
                "Exercise 04: English Pattern",
                ChapterKind::Exercise
            ),
            ("forced-guide", "练习 99：清单标为导读", ChapterKind::Guide),
            ("api-设计", "ＡＰＩ　设计", ChapterKind::Guide),
            ("api-设计-2", "API 设计", ChapterKind::Guide),
            ("setup-config", "Setup & Config", ChapterKind::Guide),
            ("setup-config-2", "Setup & Config", ChapterKind::Guide),
        ]
    );

    // Only Exercise chapters count toward total_exercises (4 exercises, 6 guides)
    assert_eq!(course.total_exercises, 4);
}

#[test]
fn command_blocks_parse_bash_sh_zsh_with_explicit_attrs_overriding_heuristics() {
    let dir = TempDir::new().expect("temp dir");
    let md_path = dir.path().join("commands.md");

    fs::write(
        &md_path,
        r#"# Command Parsing Matrix

## 练习 1：命令解析验证

```go
package main
func main() {}
```

```bash cwd="custom/svc" mode="oneshot" label="Seed database" port=9090
cd ignored-dir
go run ./cmd/seed
```

```sh mode="daemon"
echo "custom watcher"
```

```zsh
cd services/api && npm run dev
```

```bash
mkdir task-api
cd task-api
go mod init example.com/task-api
```

```bash
go run .
```

```sh
cd ../outside-escape
go test -race ./...
```

```bash role="debug"
go version
```

```bash
curl -i http://127.0.0.1:8080/healthz
```

```bash cwd="."
cd ignored-subdir
go test ./...
```

**验收：**
- 所有命令解析正确。
"#,
    )
    .expect("write commands.md");

    let app = App::open(AppConfig {
        data_dir: dir.path().join("app-data"),
    })
    .expect("open app");

    let course = match app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: md_path.to_string_lossy().into_owned(),
        }))
        .expect("open markdown")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected LoadedCourse, got {other:?}"),
    };

    let ch = &course.chapters[0];
    // Go fence is NOT a Command Block; 9 bash/sh/zsh fences ARE Command Blocks
    assert_eq!(ch.commands.len(), 9);

    // 1. Explicit cwd + mode="oneshot" override leading `cd` and `go run` daemon heuristic
    let c0 = &ch.commands[0];
    assert_eq!(c0.shell, "bash");
    assert_eq!(c0.cwd.as_deref(), Some("custom/svc"));
    assert_eq!(c0.cwd_origin, ValueOrigin::Explicit);
    assert_eq!(c0.mode, CommandMode::Oneshot);
    assert_eq!(c0.mode_origin, ValueOrigin::Explicit);
    assert_eq!(c0.label.as_deref(), Some("Seed database"));
    assert_eq!(c0.port, Some(9090));

    // 2. Explicit mode="daemon" overrides oneshot echo heuristic; c0's explicit cwd must not leak into c1
    let c1 = &ch.commands[1];
    assert_eq!(c1.shell, "sh");
    assert_eq!(c1.cwd, None);
    assert_eq!(c1.cwd_origin, ValueOrigin::WorkspaceRoot);
    assert_eq!(c1.mode, CommandMode::Daemon);
    assert_eq!(c1.mode_origin, ValueOrigin::Explicit);

    // 3. zsh leading `cd services/api && npm run dev` infers cwd + daemon
    let c2 = &ch.commands[2];
    assert_eq!(c2.shell, "zsh");
    assert_eq!(c2.cwd.as_deref(), Some("services/api"));
    assert_eq!(c2.cwd_origin, ValueOrigin::Inferred);
    assert_eq!(c2.mode, CommandMode::Daemon);
    assert_eq!(c2.mode_origin, ValueOrigin::Inferred);

    // 4. `mkdir task-api` precedes `cd task-api`: must run at workspace root (not inside services/api or task-api)
    let c3 = &ch.commands[3];
    assert_eq!(c3.mode, CommandMode::Oneshot);
    assert_eq!(
        c3.cwd, None,
        "block that scaffolds `mkdir task-api && cd task-api` must run at workspace root"
    );

    // 5. Subsequent `go run .` inherits task-api cwd and daemon mode
    let c4 = &ch.commands[4];
    assert_eq!(c4.cwd.as_deref(), Some("task-api"));
    assert_eq!(c4.cwd_origin, ValueOrigin::Inferred);
    assert_eq!(c4.mode, CommandMode::Daemon);

    // 6. `cd ../outside-escape` is ignored as unsafe relative cwd; `go test` stays oneshot
    let c5 = &ch.commands[5];
    assert_ne!(c5.cwd.as_deref(), Some("../outside-escape"));
    assert_eq!(c5.mode, CommandMode::Oneshot);

    // 7. Explicit role="debug" marks secondary
    let c6 = &ch.commands[6];
    assert!(c6.is_secondary);

    // 8. Heuristic curl marks secondary
    let c7 = &ch.commands[7];
    assert!(c7.is_secondary);
    assert_eq!(c7.mode, CommandMode::Oneshot);

    // 9. Explicit cwd="." pins workspace root and overrides leading `cd` and chapter context
    let c8 = &ch.commands[8];
    assert_eq!(c8.cwd, None);
    assert_eq!(c8.cwd_origin, ValueOrigin::Explicit);
    assert_eq!(c8.mode, CommandMode::Oneshot);
}

#[test]
fn pure_external_markdown_degrades_gracefully_preserving_document_order_without_empty_steps() {
    let dir = TempDir::new().expect("temp dir");
    let md_path = dir.path().join("plain-tutorial.md");

    fs::write(
        &md_path,
        r#"# Everyday Shell Notes

Welcome to a plain external Markdown article with zero CodeLab metadata.

## Checking Local Services

First inspect the listening socket:

```bash
lsof -i :8080
```

Then start the local dev server in the api directory:

```sh
cd api && npm run dev
```

Finally inspect logs after startup.
"#,
    )
    .expect("write plain-tutorial.md");

    let app = App::open(AppConfig {
        data_dir: dir.path().join("app-data"),
    })
    .expect("open app");

    let course = match app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: md_path.to_string_lossy().into_owned(),
        }))
        .expect("open plain markdown")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected LoadedCourse, got {other:?}"),
    };

    assert!(!course.is_builtin);
    assert!(matches!(course.source, CourseSource::LocalMarkdown { .. }));
    assert_eq!(course.title, "Everyday Shell Notes");
    assert_eq!(course.total_exercises, 0);
    assert_eq!(course.chapters.len(), 1);

    let ch = &course.chapters[0];
    assert_eq!(ch.chapter_id, "checking-local-services");
    assert_eq!(ch.kind, ChapterKind::Guide);
    assert_eq!(ch.presentation, ChapterPresentation::LinearFallback);
    assert!(
        ch.three_step.is_none(),
        "plain external Markdown must not fabricate empty three-step sections"
    );
    assert_eq!(ch.commands.len(), 2);
    assert_eq!(ch.commands[0].mode, CommandMode::Oneshot);
    assert_eq!(ch.commands[1].cwd.as_deref(), Some("api"));
    assert_eq!(ch.commands[1].mode, CommandMode::Daemon);

    // Original order preserved: Prose -> Command -> Prose -> Command -> Prose
    assert_eq!(ch.blocks.len(), 5);
    assert!(matches!(ch.blocks[0], ContentBlock::Prose { .. }));
    assert!(matches!(
        &ch.blocks[1],
        ContentBlock::Command { command_id } if command_id == &ch.commands[0].id
    ));
    assert!(matches!(ch.blocks[2], ContentBlock::Prose { .. }));
    assert!(matches!(
        &ch.blocks[3],
        ContentBlock::Command { command_id } if command_id == &ch.commands[1].id
    ));
    assert!(matches!(ch.blocks[4], ContentBlock::Prose { .. }));

    // Pre-H2 shell command in plain Markdown is promoted into an opening guide chapter so it is never lost
    let pre_h2_path = dir.path().join("pre-h2-cmd.md");
    fs::write(
        &pre_h2_path,
        "# Quickstart\n\nBootstrap first:\n\n```bash\ngo version\n```\n\n## Next Steps\n\nAll set.\n",
    )
    .expect("write pre-h2-cmd.md");
    let pre_h2_course = match app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: pre_h2_path.to_string_lossy().into_owned(),
        }))
        .expect("open pre-h2 markdown")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected LoadedCourse, got {other:?}"),
    };
    assert_eq!(pre_h2_course.chapters.len(), 2);
    assert_eq!(pre_h2_course.chapters[0].kind, ChapterKind::Guide);
    assert_eq!(pre_h2_course.chapters[0].commands.len(), 1);
    assert_eq!(pre_h2_course.chapters[0].commands[0].command, "go version");
}

#[test]
fn local_course_pack_rejects_traversal_malformed_manifest_and_malformed_checks() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().join("app-data"),
    })
    .expect("open app");

    // 1. Entry file traversal outside pack root
    let escape_pack = dir.path().join("escape-pack");
    fs::create_dir_all(&escape_pack).expect("create escape pack");
    fs::write(dir.path().join("secret.md"), "# Secret\n").expect("write outside file");
    fs::write(
        escape_pack.join("codelab.json"),
        r#"{"schemaVersion":"1.0","courseId":"escape","title":"Escape","entryFile":"../secret.md"}"#,
    )
    .expect("write escape manifest");

    let err = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
            path: escape_pack.to_string_lossy().into_owned(),
        }))
        .expect_err("traversal entryFile must fail");
    assert_eq!(err.code, ErrorCode::CoursePathInvalid);

    // 1b. Symlinked codelab.json escaping pack root must be rejected (Unix)
    #[cfg(unix)]
    {
        let symlink_pack = dir.path().join("symlink-manifest-pack");
        fs::create_dir_all(&symlink_pack).expect("create symlink pack");
        let outside_manifest = dir.path().join("outside-codelab.json");
        fs::write(
            &outside_manifest,
            r#"{"schemaVersion":"1.0","courseId":"outside","title":"Outside","entryFile":"README.md"}"#,
        )
        .expect("write outside manifest");
        std::os::unix::fs::symlink(&outside_manifest, symlink_pack.join("codelab.json"))
            .expect("create symlink");
        let sym_err = app
            .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
                path: symlink_pack.to_string_lossy().into_owned(),
            }))
            .expect_err("symlinked manifest escaping pack root must fail");
        assert_eq!(sym_err.code, ErrorCode::CoursePathInvalid);
    }

    // 2. Directory without codelab.json
    let empty_dir = dir.path().join("no-manifest");
    fs::create_dir_all(&empty_dir).expect("create dir");
    let err = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
            path: empty_dir.to_string_lossy().into_owned(),
        }))
        .expect_err("missing codelab.json must fail");
    assert_eq!(err.code, ErrorCode::CourseManifestInvalid);

    // 3. Invalid .codelab-checks.json
    let bad_checks_pack = dir.path().join("bad-checks");
    fs::create_dir_all(&bad_checks_pack).expect("create bad checks dir");
    fs::write(
        bad_checks_pack.join("codelab.json"),
        r#"{"schemaVersion":"1.0","courseId":"bad","title":"Bad Checks","entryFile":"README.md","checksFile":".codelab-checks.json"}"#,
    )
    .expect("write manifest");
    fs::write(
        bad_checks_pack.join("README.md"),
        "# Bad\n\n## Intro\nHello\n",
    )
    .expect("write readme");
    fs::write(
        bad_checks_pack.join(".codelab-checks.json"),
        "{not valid json",
    )
    .expect("write invalid checks");

    let err = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalPack {
            path: bad_checks_pack.to_string_lossy().into_owned(),
        }))
        .expect_err("invalid checks json must fail");
    assert_eq!(err.code, ErrorCode::CourseChecksInvalid);
}

#[test]
fn markdown_enforces_5mb_limit_rejects_binary_and_sanitizes_xss_payloads() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().join("app-data"),
    })
    .expect("open app");

    // 1. Over 5 MB rejected
    let huge_md = dir.path().join("huge.md");
    let oversized = vec![b'a'; MAX_MARKDOWN_BYTES + 1];
    fs::write(&huge_md, oversized).expect("write oversized file");
    let size_err = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: huge_md.to_string_lossy().into_owned(),
        }))
        .expect_err("oversized markdown must be rejected");
    assert_eq!(size_err.code, ErrorCode::CourseMarkdownTooLarge);
    assert_eq!(size_err.details.get("maxMb").map(String::as_str), Some("5"));

    // 2. Non-UTF-8 binary rejected
    let bin_md = dir.path().join("binary.md");
    fs::write(&bin_md, [0xff_u8, 0xfe, 0x00, 0x99]).expect("write binary bytes");
    let bin_err = app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: bin_md.to_string_lossy().into_owned(),
        }))
        .expect_err("invalid UTF-8 markdown must be rejected");
    assert_eq!(bin_err.code, ErrorCode::CourseMarkdownInvalid);

    // 3. Strict XSS sanitization
    let xss_md = dir.path().join("xss.md");
    fs::write(
        &xss_md,
        r#"# Security Check

## 练习 1：XSS 清洗验证

<script>alert('pwned')</script>

<img src="x" onerror="alert(1)">

[Bad JS Link](javascript:alert(1))

[Bad Data Link](data:text/html,<script>alert(2)</script>)

<svg onload="alert(3)"><script>alert(4)</script></svg>

<iframe src="https://evil.example/embed"></iframe>

<a href="https://go.dev" onclick="steal()" style="background:url(javascript:alert(5))">Go Docs</a>

```html
<script>console.log("escaped in code fence")</script>
```

**验收：**
- 页面无脚本执行风险。
"#,
    )
    .expect("write xss.md");

    let course = match app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: xss_md.to_string_lossy().into_owned(),
        }))
        .expect("open xss markdown")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected LoadedCourse, got {other:?}"),
    };

    let ch = &course.chapters[0];
    let mut all_html = String::new();
    if let Some(intro) = &course.intro_html {
        all_html.push_str(intro);
    }
    if let Some(summary) = &ch.summary_html {
        all_html.push_str(summary);
    }
    for block in &ch.blocks {
        if let ContentBlock::Prose { html } = block {
            all_html.push_str(html);
        }
    }
    if let Some(ts) = &ch.three_step {
        if let Some(s1) = &ts.step1_intro_html {
            all_html.push_str(s1);
        }
        if let Some(s2) = &ts.step2_html {
            all_html.push_str(s2);
        }
        if let Some(s3) = &ts.step3_intro_html {
            all_html.push_str(s3);
        }
        if let Some(r) = &ts.reflections_html {
            all_html.push_str(r);
        }
    }

    let lower = all_html.to_ascii_lowercase();
    for forbidden in [
        "<script",
        "<iframe",
        "<svg",
        "onerror=",
        "onload=",
        "onclick=",
        "javascript:",
        "data:text/html",
        "style=",
    ] {
        assert!(
            !lower.contains(forbidden),
            "sanitized HTML must not contain `{forbidden}`: {all_html}"
        );
    }

    // Safe external link preserved with noopener noreferrer; fenced code block stays escaped
    assert!(all_html.contains("https://go.dev"));
    assert!(all_html.contains("noopener noreferrer"));
    assert!(all_html.contains("&lt;script&gt;console.log"));
}

#[test]
fn thousand_line_markdown_parses_within_150ms_budget() {
    let dir = TempDir::new().expect("temp dir");
    let md_path = dir.path().join("large-handbook.md");

    let mut content = String::from("# Performance Benchmark Course\n\n");
    for chapter_num in 0..30 {
        content.push_str(&format!(
            "## 练习 {chapter_num}：性能压测章节 {chapter_num}\n\n"
        ));
        content.push_str("实现本节目标并运行测试验证吞吐量与稳定性。\n\n");
        for p in 0..6 {
            content.push_str(&format!(
                "- 要点 {p}：校验输入边界、错误码与并发安全，`ctx.Done()` 需及时响应。\n"
            ));
        }
        content.push_str("\n```go\npackage bench\n\nfunc Run(n int) int {\n    total := 0\n    for i := 0; i < n; i++ {\n        total += i\n    }\n    return total\n}\n```\n\n");
        content.push_str(&format!(
            "```bash cwd=\"pkg-{chapter_num}\"\ngo test -race ./...\n```\n\n"
        ));
        content.push_str("```bash\ncurl -i http://127.0.0.1:8080/healthz\n```\n\n");
        content.push_str("**验收：**\n- 所有测试通过。\n- 无竞态告警。\n\n");
        content.push_str("**思考：**\n- 为什么需要在边界处限制输入体积？\n\n");
    }

    let line_count = content.lines().count();
    assert!(
        line_count >= 1000,
        "benchmark document must have >= 1000 lines (got {line_count})"
    );
    fs::write(&md_path, content).expect("write benchmark markdown");

    let app = App::open(AppConfig {
        data_dir: dir.path().join("app-data"),
    })
    .expect("open app");

    let started = Instant::now();
    let course = match app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: md_path.to_string_lossy().into_owned(),
        }))
        .expect("parse 1000-line markdown")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected LoadedCourse, got {other:?}"),
    };
    let elapsed = started.elapsed();

    assert_eq!(course.total_exercises, 30);
    assert!(
        elapsed <= Duration::from_millis(150),
        "1,000+ line Markdown ({line_count} lines) took {elapsed:?}, exceeding 150ms budget"
    );
}

#[test]
fn local_markdown_external_edit_emits_hot_reload_event_and_reload_preserves_chapter_id() {
    let dir = TempDir::new().expect("temp dir");
    let md_path = dir.path().join("live-manual.md");
    fs::write(
        &md_path,
        r#"# Live Course

## 练习 1：初版标题 {#stable-ch}
Original instructions v1.

```bash
go test ./...
```

**验收：**
- 初版验收条件
"#,
    )
    .expect("write initial markdown");

    let app = App::open(AppConfig {
        data_dir: dir.path().join("app-data"),
    })
    .expect("open app");
    let events = app.subscribe();

    // Reloading before opening any course yields NoActiveCourse
    let no_active = app
        .execute(AppCommand::ReloadActiveCourse)
        .expect_err("reload without active course must fail");
    assert_eq!(no_active.code, ErrorCode::NoActiveCourse);

    let initial = match app
        .execute(AppCommand::OpenCourse(OpenCourseRequest::LocalMarkdown {
            path: md_path.to_string_lossy().into_owned(),
        }))
        .expect("open local markdown")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected LoadedCourse, got {other:?}"),
    };
    let drained: Vec<_> = std::iter::from_fn(|| events.try_recv().ok()).collect();
    assert!(
        drained
            .iter()
            .any(|event| matches!(event, AppEvent::CourseLoaded(_))),
        "expected initial CourseLoaded event, saw: {drained:?}"
    );
    assert_eq!(initial.chapters[0].chapter_id, "stable-ch");

    // Simulate external editor updating the local Markdown file on disk
    fs::write(
        &md_path,
        r#"# Live Course

## 练习 1：修订后的标题 {#stable-ch}
Updated instructions v2 after hot reload.

```bash
go test -race ./...
```

**验收：**
- 更新后的验收条件
"#,
    )
    .expect("externally modify markdown");

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    let changed = loop {
        match events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(AppEvent::CourseSourceChanged(ev)) => break ev,
            Ok(other) => seen.push(other),
            Err(error) => {
                panic!(
                    "CourseSourceChanged event must fire on external edit: {error}; saw: {seen:?}"
                );
            }
        }
    };
    assert_eq!(changed.course_id, initial.course_id);
    assert!(!changed.changed_paths.is_empty());

    // User confirms hot reload
    let reloaded = match app
        .execute(AppCommand::ReloadActiveCourse)
        .expect("reload active course")
    {
        AppResponse::LoadedCourse(c) => c,
        other => panic!("expected reloaded LoadedCourse, got {other:?}"),
    };
    assert_eq!(reloaded.chapters[0].chapter_id, "stable-ch");
    assert_eq!(reloaded.chapters[0].title, "练习 1：修订后的标题");
    assert!(
        reloaded.chapters[0]
            .summary_html
            .as_deref()
            .unwrap_or_default()
            .contains("Updated instructions v2")
    );
    assert!(reloaded.chapters[0].commands[0].command.contains("-race"));

    // Closing active course stops watching cleanly
    assert_eq!(
        app.execute(AppCommand::CloseActiveCourse)
            .expect("close active course"),
        AppResponse::CourseClosed
    );
    let err_after_close = app
        .execute(AppCommand::ReloadActiveCourse)
        .expect_err("reload after close must fail");
    assert_eq!(err_after_close.code, ErrorCode::NoActiveCourse);
}

#[test]
fn builtin_four_language_catalog_exposes_ts_python_and_java_course_packs() {
    let dir = TempDir::new().expect("temp dir");
    let app = App::open(AppConfig {
        data_dir: dir.path().to_path_buf(),
    })
    .expect("open app");

    let catalog = match app
        .execute(AppCommand::ListBuiltinCourses)
        .expect("list builtin courses")
    {
        AppResponse::BuiltinCourses(list) => list,
        other => panic!("expected BuiltinCourses, got {other:?}"),
    };
    assert_eq!(catalog.len(), 4);

    for (course_id, language, expected_exercises, expected_timeout) in [
        ("go-goframe-practice", CourseLanguage::Go, 10_u32, None),
        ("ts-data-pipeline", CourseLanguage::JsTs, 8_u32, Some(5_u64)),
        (
            "python-data-clean",
            CourseLanguage::Python,
            6_u32,
            Some(8_u64),
        ),
        (
            "java-rate-limiter",
            CourseLanguage::Java,
            7_u32,
            Some(12_u64),
        ),
    ] {
        let summary = catalog
            .iter()
            .find(|c| c.course_id == course_id)
            .unwrap_or_else(|| panic!("missing builtin course {course_id}"));
        assert_eq!(summary.language, Some(language));
        assert_eq!(summary.total_exercises, expected_exercises);
        assert!(summary.is_builtin);

        let loaded = match app
            .execute(AppCommand::OpenCourse(OpenCourseRequest::Builtin {
                course_id: course_id.into(),
            }))
            .expect("open builtin course")
        {
            AppResponse::LoadedCourse(course) => course,
            other => panic!("expected LoadedCourse, got {other:?}"),
        };
        assert_eq!(loaded.language, Some(language));
        assert_eq!(loaded.total_exercises, expected_exercises);
        assert!(loaded.trusted);

        if let Some(timeout) = expected_timeout {
            let exercises: Vec<_> = loaded
                .chapters
                .iter()
                .filter(|ch| ch.kind == ChapterKind::Exercise)
                .collect();
            assert_eq!(exercises.len() as u32, expected_exercises);
            for (idx, chapter) in exercises.iter().enumerate() {
                assert_eq!(chapter.chapter_id, format!("ex-{}", idx + 1));
                assert_eq!(chapter.presentation, ChapterPresentation::ThreeStep);
                let step = chapter
                    .three_step
                    .as_ref()
                    .expect("exercise three-step projection");
                assert!(
                    step.step3_checks.iter().any(|check| {
                        check.kind == CheckKind::TestSuite
                            && check.timeout_sec == Some(timeout)
                            && !check.require_tests.is_empty()
                    }),
                    "{course_id} {} must declare test_suite check with timeout {timeout}s",
                    chapter.chapter_id
                );
            }
        }
    }
}
