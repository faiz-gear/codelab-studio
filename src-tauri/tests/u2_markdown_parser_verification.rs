use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use codelab_studio_lib::course::{
    parse_markdown, parse_markdown_file, ParseMarkdownOptions, MAX_MARKDOWN_BYTES,
};
use codelab_studio_lib::domain::course::MarkdownNode;
use codelab_studio_lib::domain::error::ErrorCode;
use codelab_studio_lib::domain::ids::{ChapterId, CommandBlockId, RelWorkspacePath};
use codelab_studio_lib::domain::pty::PtyRole;
use codelab_studio_lib::domain::runtime::Language;

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn count_nodes<F>(nodes: &[MarkdownNode], predicate: &F) -> usize
where
    F: Fn(&MarkdownNode) -> bool,
{
    let mut total = 0usize;
    for node in nodes {
        if predicate(node) {
            total += 1;
        }
        total += match node {
            MarkdownNode::Paragraph { children }
            | MarkdownNode::Heading { children, .. }
            | MarkdownNode::Strong { children }
            | MarkdownNode::Emphasis { children }
            | MarkdownNode::BlockQuote { children }
            | MarkdownNode::Link { children, .. }
            | MarkdownNode::Superscript { children }
            | MarkdownNode::Subscript { children }
            | MarkdownNode::Details { children, .. } => count_nodes(children, predicate),
            MarkdownNode::List { items, .. } => {
                items.iter().map(|item| count_nodes(item, predicate)).sum()
            }
            MarkdownNode::Table { headers, rows } => {
                headers
                    .iter()
                    .map(|cell| count_nodes(cell, predicate))
                    .sum::<usize>()
                    + rows
                        .iter()
                        .map(|row| {
                            row.iter()
                                .map(|cell| count_nodes(cell, predicate))
                                .sum::<usize>()
                        })
                        .sum::<usize>()
            }
            MarkdownNode::Text { .. }
            | MarkdownNode::InlineCode { .. }
            | MarkdownNode::CodeBlock { .. }
            | MarkdownNode::Image { .. }
            | MarkdownNode::LineBreak
            | MarkdownNode::ThematicBreak
            | MarkdownNode::Kbd { .. } => 0,
        };
    }
    total
}

fn contains_node<F>(nodes: &[MarkdownNode], predicate: &F) -> bool
where
    F: Fn(&MarkdownNode) -> bool,
{
    count_nodes(nodes, predicate) > 0
}

#[test]
fn go_handbook_fixture_parses_all_10_chapters_and_three_step_flows() {
    let path = fixture_path("go_handbook_sample.md");
    let course = parse_markdown_file(&path, &ParseMarkdownOptions::default())
        .expect("parse go_handbook_sample.md");

    assert_eq!(course.title, "Go & GoFrame 工程实战手册");
    assert_eq!(course.course_id, "go-goframe-工程实战手册");
    assert_eq!(course.language, Some(Language::Go));
    assert!(
        !course.intro_nodes.is_empty(),
        "prelude between H1 and first H2 must be preserved"
    );
    assert!(contains_node(&course.intro_nodes, &|n| matches!(
        n,
        MarkdownNode::Table { .. }
    )));
    assert!(contains_node(&course.intro_nodes, &|n| matches!(
        n,
        MarkdownNode::Link { href, .. } if href == "https://go.dev/doc/"
    )));

    assert_eq!(course.chapters.len(), 10);
    for (idx, chapter) in course.chapters.iter().enumerate() {
        assert_eq!(chapter.index, idx as u32);
        assert!(
            chapter
                .chapter_id
                .0
                .starts_with(&format!("ch-{:02}-", idx + 1)),
            "unexpected chapter_id '{}'",
            chapter.chapter_id.0
        );
    }

    let ch0 = &course.chapters[0];
    assert!(ch0.chapter_id.0.contains("练习-0"));
    assert_eq!(ch0.command_blocks.len(), 2);
    assert_eq!(ch0.command_blocks[0].cwd, None);
    assert!(ch0.command_blocks[0].is_secondary);
    assert_eq!(ch0.command_blocks[1].cwd, None);
    assert!(
        !ch0.command_blocks[1].is_secondary,
        "first non-pure-cd command should be promoted when chapter has no daemon or test runner"
    );
    assert_eq!(ch0.three_step.step1_primary_commands.len(), 1);
    assert_eq!(ch0.three_step.secondary_commands.len(), 1);
    assert_eq!(ch0.self_check_items.len(), 1);
    assert_eq!(ch0.three_step.step3_self_checks.len(), 1);
    assert_eq!(ch0.three_step.reflection_nodes.len(), 1);
    assert!(contains_node(&ch0.nodes, &|n| matches!(
        n,
        MarkdownNode::Details { summary, .. } if summary == "思考"
    )));

    let ch1 = &course.chapters[1];
    assert_eq!(ch1.three_step.step2_reference_blocks.len(), 1);
    assert_eq!(
        ch1.three_step.step2_reference_blocks[0]
            .target_path
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("task-api/main.go")
    );
    assert!(contains_node(&ch1.nodes, &|n| matches!(
        n,
        MarkdownNode::CodeBlock {
            target_path: Some(p),
            ..
        } if p.as_str() == "task-api/main.go"
    )));

    assert_eq!(ch1.command_blocks.len(), 3);
    assert!(ch1.command_blocks[0].is_secondary);
    assert_eq!(
        ch1.command_blocks[1]
            .cwd
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("task-api")
    );
    assert!(!ch1.command_blocks[1].is_secondary);
    match &ch1.command_blocks[1].role {
        PtyRole::Daemon {
            service_key,
            label,
            port_hint,
        } => {
            assert_eq!(service_key, "task-api");
            assert_eq!(label, "go run .");
            assert_eq!(*port_hint, Some(8080));
        }
        other => panic!("expected Daemon role for go run ., got {other:?}"),
    }
    assert_eq!(
        ch1.command_blocks[2]
            .cwd
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("task-api")
    );
    assert!(ch1.command_blocks[2].is_secondary);
    assert_eq!(ch1.self_check_items.len(), 2);
    assert_eq!(ch1.three_step.step3_self_checks[0].id, "s1");
    assert_eq!(ch1.three_step.step3_self_checks[1].id, "s2");

    let ch2 = &course.chapters[2];
    assert_eq!(
        ch2.three_step.step2_reference_blocks[0]
            .target_path
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("task-api/internal/handler.go")
    );
    assert!(!ch2.command_blocks[0].is_secondary);

    let ch3 = &course.chapters[3];
    assert_eq!(
        ch3.three_step.step2_reference_blocks[0]
            .target_path
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("task-api/store.go")
    );
    assert!(!ch3.command_blocks[1].is_secondary);

    let ch4 = &course.chapters[4];
    assert_eq!(
        ch4.command_blocks.len(),
        2,
        "nested curl inside ordered list must be extracted"
    );
    assert_eq!(ch4.command_blocks[0].cwd, None);
    assert!(matches!(
        ch4.command_blocks[0].role,
        PtyRole::Daemon {
            port_hint: Some(8080),
            ..
        }
    ));
    let nested_cmd_id = ch4.command_blocks[1].id.clone();
    assert_eq!(
        ch4.command_blocks[1]
            .cwd
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("task-api")
    );
    assert!(ch4.command_blocks[1]
        .command
        .contains("curl --max-time 0.2"));
    assert!(contains_node(&ch4.nodes, &|n| matches!(
        n,
        MarkdownNode::CodeBlock {
            command_block_id: Some(id),
            ..
        } if *id == nested_cmd_id
    )));

    let ch8 = &course.chapters[8];
    assert_eq!(ch8.command_blocks.len(), 3);
    assert_eq!(ch8.command_blocks[1].cwd, None);
    assert!(!ch8.command_blocks[1].is_secondary);
    match &ch8.command_blocks[1].role {
        PtyRole::Daemon {
            service_key,
            port_hint,
            ..
        } => {
            assert_eq!(service_key, "task-api-gf");
            assert_eq!(*port_hint, Some(8000));
        }
        other => panic!("expected GoFrame daemon role, got {other:?}"),
    }
    assert_eq!(
        ch8.command_blocks[2]
            .cwd
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("task-api-gf")
    );
    assert!(!ch8.three_step.reflection_nodes.is_empty());
    assert!(contains_node(&ch8.nodes, &|n| matches!(
        n,
        MarkdownNode::Details { summary, .. } if summary == "课后思考"
    )));

    let ch9 = &course.chapters[9];
    assert_eq!(ch9.self_check_items.len(), 3);
    assert_eq!(ch9.three_step.step3_self_checks.len(), 3);
}

#[test]
fn external_attrs_safe_html_and_multilang_daemons() {
    let path = fixture_path("external_markdown_attrs_and_html.md");
    let course = parse_markdown_file(&path, &ParseMarkdownOptions::default())
        .expect("parse external_markdown_attrs_and_html.md");

    assert_eq!(course.title, "Multi-Stack Full Course");
    assert!(contains_node(&course.intro_nodes, &|n| matches!(
        n,
        MarkdownNode::Kbd { label } if label == "Cmd"
    )));
    assert!(contains_node(&course.intro_nodes, &|n| matches!(
        n,
        MarkdownNode::Superscript { .. }
    )));
    assert!(contains_node(&course.intro_nodes, &|n| matches!(
        n,
        MarkdownNode::Subscript { .. }
    )));
    assert!(contains_node(&course.intro_nodes, &|n| matches!(
        n,
        MarkdownNode::LineBreak
    )));

    assert_eq!(course.chapters.len(), 3);
    assert_eq!(
        course.chapters[0].chapter_id,
        ChapterId("ex-1-frontend".to_owned())
    );
    assert_eq!(course.chapters[0].title, "Frontend Dev Setup");
    assert_eq!(
        course.chapters[1].chapter_id,
        ChapterId("dup-anchor".to_owned())
    );
    assert_eq!(
        course.chapters[2].chapter_id,
        ChapterId("dup-anchor-2".to_owned())
    );

    let fe = &course.chapters[0];
    assert_eq!(fe.command_blocks.len(), 3);
    assert_eq!(fe.command_blocks[0].label.as_deref(), Some("Start Web Dev"));
    assert_eq!(
        fe.command_blocks[0]
            .cwd
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("web-app")
    );
    assert!(!fe.command_blocks[0].is_secondary);
    assert_eq!(
        fe.command_blocks[0].role,
        PtyRole::Daemon {
            service_key: "web-ui".to_owned(),
            label: "Start Web Dev".to_owned(),
            port_hint: Some(5173),
        }
    );
    assert!(fe.command_blocks[1].is_secondary);
    assert!(!fe.command_blocks[2].is_secondary);
    assert_eq!(
        fe.three_step.step2_reference_blocks[0]
            .target_path
            .as_ref()
            .map(RelWorkspacePath::as_str),
        Some("web-app/src/index.ts")
    );
    assert!(contains_node(&fe.nodes, &|n| matches!(
        n,
        MarkdownNode::Details { summary, children }
            if summary == "展开提示"
                && contains_node(
                    children,
                    &|inner| matches!(inner, MarkdownNode::Kbd { label } if label == "K"),
                )
    )));
    assert_eq!(fe.three_step.step3_self_checks.len(), 2);
    assert!(!fe.three_step.step3_self_checks[0].checked_by_default);
    assert!(fe.three_step.step3_self_checks[1].checked_by_default);

    let py = &course.chapters[1];
    assert_eq!(py.command_blocks.len(), 2);
    assert!(matches!(
        py.command_blocks[0].role,
        PtyRole::Daemon {
            port_hint: Some(8000),
            ..
        }
    ));
    assert!(!py.command_blocks[0].is_secondary);
    assert!(!py.command_blocks[1].is_secondary);
    assert_eq!(
        py.three_step.step2_reference_blocks[0].target_path, None,
        "path traversal attribute must be rejected cleanly"
    );
    assert!(!py.three_step.reflection_nodes.is_empty());
    assert!(contains_node(&py.nodes, &|n| matches!(
        n,
        MarkdownNode::Details { summary, .. } if summary == "Reflection"
    )));

    let java = &course.chapters[2];
    assert_eq!(java.command_blocks.len(), 2);
    assert!(matches!(
        java.command_blocks[0].role,
        PtyRole::Daemon { .. }
    ));
    assert!(!java.command_blocks[0].is_secondary);
    assert!(!java.command_blocks[1].is_secondary);
    assert_eq!(java.three_step.step3_self_checks.len(), 1);
    assert!(java.three_step.step3_self_checks[0]
        .title
        .contains("actuator/health"));
}

#[test]
fn zero_h2_document_and_extra_h1_preserve_all_content() {
    let md = concat!(
        "# Standalone Guide\n\n",
        "Follow these steps.\n\n",
        "# Extra Heading\n\n",
        "```bash\n",
        "go test ./...\n",
        "```\n",
    );
    let course = parse_markdown(md, &ParseMarkdownOptions::default()).expect("parse zero-H2 doc");

    assert_eq!(course.title, "Standalone Guide");
    assert!(course.intro_nodes.is_empty());
    assert_eq!(course.chapters.len(), 1);
    assert_eq!(course.chapters[0].title, "Standalone Guide");
    assert_eq!(course.chapters[0].command_blocks.len(), 1);
    assert!(contains_node(&course.chapters[0].nodes, &|n| matches!(
        n,
        MarkdownNode::Heading { level: 1, .. }
    )));
}

#[test]
fn contextual_cd_tracking_enforces_sandbox_and_avoids_double_cd() {
    let md = r#"# CWD Tracking

## Sandbox Rules

```bash
mkdir -p app/sub
cd app/sub
echo init
```

```bash
pwd
```

```bash
cd ..
```

```bash
pwd
```

```bash
cd ../../../outside
```

```bash
pwd
```

```bash
cd app/sub
```

```bash
pwd
```

```bash
cd /etc
```

```bash
pwd
```

```bash
cd app/sub
```

```bash
pwd
```

```bash
cd ~
```

```bash
pwd
```

```bash
cd app/sub
```

```bash
pwd
```

```bash
cd $HOME
```

```bash
pwd
```

```bash
cd app/sub
```

```bash
cd -
```

```bash
pwd
```
"#;

    let course = parse_markdown(md, &ParseMarkdownOptions::default()).expect("parse cwd test");
    let cmds = &course.chapters[0].command_blocks;

    assert_eq!(cmds[0].cwd, None);
    assert_eq!(
        cmds[1].cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("app/sub")
    );
    assert_eq!(
        cmds[2].cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("app/sub")
    );
    assert_eq!(
        cmds[3].cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("app")
    );
    assert_eq!(
        cmds[4].cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("app")
    );
    assert_eq!(cmds[5].cwd, None, "cd ../../../outside must reset to root");

    assert_eq!(
        cmds[7].cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("app/sub")
    );
    assert_eq!(
        cmds[8].cwd, None,
        "leading cd /etc resets block cwd to root"
    );
    assert_eq!(
        cmds[9].cwd, None,
        "cd /etc must keep post-block cwd at root"
    );

    assert_eq!(
        cmds[11].cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("app/sub")
    );
    assert_eq!(cmds[12].cwd, None, "leading cd ~ resets block cwd to root");
    assert_eq!(cmds[13].cwd, None, "cd ~ must keep post-block cwd at root");

    assert_eq!(
        cmds[15].cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("app/sub")
    );
    assert_eq!(
        cmds[16].cwd, None,
        "leading cd $HOME resets block cwd to root"
    );
    assert_eq!(
        cmds[17].cwd, None,
        "cd $HOME must keep post-block cwd at root"
    );

    assert_eq!(
        cmds[19].cwd.as_ref().map(RelWorkspacePath::as_str),
        Some("app/sub")
    );
    assert_eq!(cmds[20].cwd, None, "cd - must reset non-root cwd");
}

#[test]
fn goal_extraction_handles_dotted_identifiers_and_skips_acceptance_reflection() {
    let md = r#"# Goal Test

## Chapter One

**验收：**Ignore this acceptance line for goal.

**思考：**Ignore this reflection line for goal.

Run `go run .` using `go.mod` and `database/sql` on `127.0.0.1:8080`. Second sentence should not be included.
"#;

    let course = parse_markdown(md, &ParseMarkdownOptions::default()).expect("parse goal test");
    assert_eq!(
        course.chapters[0].goal.as_deref(),
        Some("Run go run . using go.mod and database/sql on 127.0.0.1:8080.")
    );
}

#[test]
fn xss_payloads_and_unsafe_protocols_are_completely_neutralized() {
    let md = r#"# Security Audit

## XSS Vectors

<script>alert(1)</script>

<script type="text/javascript">
alert(2)
</script>

Safe before <script>alert(3)</script> safe after.

Space tag <script >alert(11)</script> still stripped.

<details data-x="a>b"><summary>Quoted gt</summary>Safe details body</details>

Style test <style>body{display:none}</style> visible.

<img src=x onerror=alert(4)>

<svg onload=alert(5)></svg>

<iframe src="https://evil.invalid"></iframe>

Shortcut <kbd onclick="evil()">⌘S</kbd> works.

<div onclick="evil()">plain div text</div>

[click1](javascript:alert(6))
[click2](  JaVaScRiPt:alert(7))
[click2b](<  JaVaScRiPt:alert(7)>)
[click3](data:text/html,<script>alert(8)</script>)
[click4](vbscript:msgbox(9))
![badimg](javascript:alert(10))
![badrel](../secret.png)

[good-https](https://example.com)
[good-mail](mailto:hi@example.com)
[good-anchor](#xss-vectors)
![good-rel](assets/arch.png)
![good-remote](https://example.com/a.png)
"#;

    let course = parse_markdown(md, &ParseMarkdownOptions::default()).expect("parse xss test");
    let json = serde_json::to_string(&course).expect("serialize AST");
    let lower_json = json.to_ascii_lowercase();

    for forbidden in [
        "<script",
        "alert(",
        "onerror",
        "onload",
        "onclick",
        "javascript:",
        "vbscript:",
        "data:text/html",
        "<iframe",
        "<svg",
        "display:none",
        "../secret.png",
    ] {
        assert!(
            !lower_json.contains(forbidden),
            "serialized AST must not contain '{forbidden}', got:\n{json}"
        );
    }

    let nodes = &course.chapters[0].nodes;
    assert!(contains_node(nodes, &|n| matches!(
        n,
        MarkdownNode::Kbd { label } if label == "⌘S"
    )));
    assert!(json.contains("Safe before"));
    assert!(json.contains("safe after"));
    assert!(json.contains("still stripped"));
    assert!(contains_node(nodes, &|n| matches!(
        n,
        MarkdownNode::Details { summary, children }
            if summary == "Quoted gt"
                && contains_node(children, &|c| matches!(
                    c,
                    MarkdownNode::Text { value } if value.contains("Safe details body")
                ))
    )));
    assert!(json.contains("plain div text"));
    assert!(json.contains("click1"));
    assert!(json.contains("click2"));
    assert!(json.contains("badimg"));
    assert_eq!(
        count_nodes(nodes, &|n| matches!(n, MarkdownNode::Link { .. })),
        3,
        "only the 3 allowlisted links may remain as Link nodes"
    );
    assert_eq!(
        count_nodes(nodes, &|n| matches!(n, MarkdownNode::Image { .. })),
        2,
        "only the 2 allowlisted images may remain as Image nodes"
    );
    assert!(contains_node(nodes, &|n| matches!(
        n,
        MarkdownNode::Text { value } if value.contains("click1") && value.contains("badimg")
    )));
    assert!(contains_node(nodes, &|n| matches!(
        n,
        MarkdownNode::Link { href, .. } if href == "https://example.com"
    )));
    assert!(contains_node(nodes, &|n| matches!(
        n,
        MarkdownNode::Link { href, .. } if href == "mailto:hi@example.com"
    )));
    assert!(contains_node(nodes, &|n| matches!(
        n,
        MarkdownNode::Link { href, .. } if href == "#xss-vectors"
    )));
    assert!(contains_node(nodes, &|n| matches!(
        n,
        MarkdownNode::Image { src, .. } if src == "assets/arch.png"
    )));
    assert!(contains_node(nodes, &|n| matches!(
        n,
        MarkdownNode::Image { src, .. } if src == "https://example.com/a.png"
    )));
}

#[test]
fn five_megabyte_limit_enforced_for_memory_and_file_inputs() {
    let oversized = "a".repeat(MAX_MARKDOWN_BYTES + 1);
    let err = parse_markdown(&oversized, &ParseMarkdownOptions::default())
        .expect_err("oversized in-memory markdown must fail");
    assert_eq!(err.code, ErrorCode::MarkdownLimitExceeded);
    assert_eq!(err.message_key, "error.markdownLimitExceeded");
    assert_eq!(
        err.params.get("maxBytes").map(String::as_str),
        Some("5242880")
    );
    assert_eq!(
        err.params.get("actualBytes").map(String::as_str),
        Some("5242881")
    );

    let exact_limit = "a".repeat(MAX_MARKDOWN_BYTES);
    let exact_ok = parse_markdown(&exact_limit, &ParseMarkdownOptions::default());
    assert!(
        exact_ok.is_ok(),
        "exact MAX_MARKDOWN_BYTES document must succeed"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let big_file = dir.path().join("too_big.md");
    fs::write(&big_file, vec![b'x'; MAX_MARKDOWN_BYTES + 1]).expect("write big file");
    let file_err = parse_markdown_file(&big_file, &ParseMarkdownOptions::default())
        .expect_err("oversized file must fail before reading");
    assert_eq!(file_err.code, ErrorCode::MarkdownLimitExceeded);

    let binary_file = dir.path().join("binary.md");
    fs::write(&binary_file, [0xff_u8, 0xfe, 0x00, 0x80]).expect("write invalid utf8");
    let utf8_err = parse_markdown_file(&binary_file, &ParseMarkdownOptions::default())
        .expect_err("invalid UTF-8 file must fail");
    assert_eq!(utf8_err.code, ErrorCode::FileTooLargeOrBinary);

    let pack_dir = dir.path().join("go-goframe-learning");
    fs::create_dir_all(&pack_dir).expect("create pack dir");
    let readme_with_h1 = pack_dir.join("README.md");
    fs::write(
        &readme_with_h1,
        "# Real Course Title\n\n## Step 1\n\nHello.\n",
    )
    .expect("write readme with h1");
    let parsed_with_h1 = parse_markdown_file(&readme_with_h1, &ParseMarkdownOptions::default())
        .expect("parse readme with h1");
    assert_eq!(parsed_with_h1.title, "Real Course Title");
    assert_eq!(parsed_with_h1.course_id, "real-course-title");

    fs::write(&readme_with_h1, "## Step 1\n\nNo H1 present.\n").expect("write readme without h1");
    let parsed_without_h1 = parse_markdown_file(&readme_with_h1, &ParseMarkdownOptions::default())
        .expect("parse readme without h1");
    assert_eq!(parsed_without_h1.title, "go-goframe-learning");
    assert_eq!(parsed_without_h1.course_id, "go-goframe-learning");
}

#[test]
fn thousand_line_realistic_markdown_parses_completely_and_quickly() {
    let mut doc = String::from("# Large Performance Course\n\nIntro overview.\n\n");
    let chapter_count = 25usize;

    for idx in 1..=chapter_count {
        doc.push_str(&format!(
            "## Exercise {idx}: Service Step {idx}\n\n\
             Build and verify service module {idx} listening on `127.0.0.1:8080`.\n\n\
             Additional context line one for exercise {idx}.\n\
             Additional context line two for exercise {idx}.\n\n\
             | Field | Value |\n\
             | --- | --- |\n\
             | Step | `{idx}` |\n\
             | Port | `8080` |\n\n\
             Create `service/step_{idx}.go`:\n\n\
             ```go\n\
             package service\n\n\
             import \"fmt\"\n\n\
             func Step{idx}() string {{\n\
             \treturn fmt.Sprintf(\"step-%d\", {idx})\n\
             }}\n\
             ```\n\n\
             ```bash cwd=service\n\
             go run .\n\
             ```\n\n\
             ```bash cwd=service\n\
             curl -s http://127.0.0.1:8080/healthz\n\
             ```\n\n\
             ```bash cwd=service\n\
             go test -race ./...\n\
             ```\n\n\
             ### Verification\n\n\
             - [ ] Endpoint `/healthz` returns 200 for step {idx}\n\
             - [ ] `go test -race ./...` passes for step {idx}\n\n\
             **Reflection:** Why does step {idx} isolate handler logic from transport wiring?\n\n"
        ));
    }

    let line_count = doc.lines().count();
    assert!(
        line_count >= 1000,
        "expected >= 1000-line fixture, got {line_count} lines"
    );

    let started = Instant::now();
    let parsed =
        parse_markdown(&doc, &ParseMarkdownOptions::default()).expect("parse 1000-line markdown");
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    let total_commands: usize = parsed
        .chapters
        .iter()
        .map(|chapter| chapter.command_blocks.len())
        .sum();
    let total_refs: usize = parsed
        .chapters
        .iter()
        .map(|chapter| chapter.three_step.step2_reference_blocks.len())
        .sum();
    let total_checks: usize = parsed
        .chapters
        .iter()
        .map(|chapter| chapter.three_step.step3_self_checks.len())
        .sum();
    let total_reflections: usize = parsed
        .chapters
        .iter()
        .map(|chapter| chapter.three_step.reflection_nodes.len())
        .sum();

    println!(
        "1000-line parse: lines={line_count} bytes={} elapsed={elapsed_ms:.3}ms \
         chapters={} commands={total_commands} refs={total_refs}",
        doc.len(),
        parsed.chapters.len()
    );

    assert_eq!(parsed.language, Some(Language::Go));
    assert_eq!(parsed.chapters.len(), chapter_count);
    assert_eq!(total_commands, chapter_count * 3);
    assert_eq!(total_refs, chapter_count);
    assert_eq!(total_checks, chapter_count * 2);
    assert_eq!(total_reflections, chapter_count);
    assert_eq!(
        parsed.chapters[0].command_blocks[0].id,
        CommandBlockId("ch-01-exercise-1-service-step-1-cmd-01".to_owned())
    );
}
