use std::path::Path;

use codelab_studio_lib::domain::course::{CourseSourceInput, MarkdownNode};
use codelab_studio_lib::domain::pty::PtyRole;
use codelab_studio_lib::platform::CodelabHomePaths;
use codelab_studio_lib::session::{AppState, SessionLifecycleKind};

#[tokio::test]
async fn inspect_local_handbook_preserves_preview_content_without_opening_workspace() {
    let home = tempfile::tempdir().unwrap();
    let state = AppState::with_home_paths(CodelabHomePaths::from_root(home.path().to_owned()));
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/course_preview.md");
    let sources = state
        .inspect_course_source(CourseSourceInput::LocalMarkdown {
            file_path: fixture.to_string_lossy().into_owned(),
        })
        .await
        .unwrap();
    let course = sources.course;
    assert_eq!(course.title, "Preview fixture");
    assert_eq!(course.intro_nodes.len(), 1);
    let [first, second] = course.chapters.as_slice() else {
        panic!("expected two chapters");
    };
    assert_eq!(first.title, "Exercise 0: Start service");
    assert_eq!(second.title, "Exercise 1: Check output");
    assert_eq!(first.nodes.len(), 6);
    assert_eq!(second.nodes.len(), 5);
    assert!(second
        .nodes
        .iter()
        .any(|node| matches!(node, MarkdownNode::Table { .. })));
    assert!(second
        .nodes
        .iter()
        .any(|node| matches!(node, MarkdownNode::Details { .. })));
    let [primary, secondary] = first.command_blocks.as_slice() else {
        panic!("expected two commands");
    };
    assert_eq!(primary.command, "npm run dev");
    assert_eq!(primary.cwd.as_ref().unwrap().as_str(), "web-app");
    assert!(matches!(primary.role, PtyRole::Daemon { .. }));
    assert!(!primary.is_secondary);
    assert!(matches!(secondary.role, PtyRole::Oneshot { .. }));
    assert!(secondary.is_secondary);
    let serialized = serde_json::to_string(&course).unwrap();
    assert!(!serialized.contains("CHECK_SCRIPT_REMOVED"));
    assert!(!serialized.contains("<script>"));
    assert!(matches!(
        state.lifecycle_state(),
        SessionLifecycleKind::TrustPending { .. }
    ));
}
