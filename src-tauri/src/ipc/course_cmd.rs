use std::path::PathBuf;

use crate::course::{preview_author_draft, save_author_pack};
use crate::domain::course::{
    AuthorPreviewResult, CourseInspectResult, CourseSourceInput, OpenWorkspaceReq,
    OpenedWorkspaceView, ParsedCourse,
};
use crate::domain::error::{AppError, ErrorCode};
use crate::domain::ids::SessionEpoch;
use crate::session::AppState;

#[tauri::command]
pub async fn course_inspect_source(
    source: CourseSourceInput,
    state: tauri::State<'_, AppState>,
) -> Result<CourseInspectResult, AppError> {
    state.inspect_course_source(source).await
}

#[tauri::command]
pub async fn course_trust_and_open(
    req: OpenWorkspaceReq,
    state: tauri::State<'_, AppState>,
) -> Result<OpenedWorkspaceView, AppError> {
    state.trust_and_open_course(req).await
}

#[tauri::command]
pub async fn course_reload_manual(
    epoch: SessionEpoch,
    state: tauri::State<'_, AppState>,
) -> Result<ParsedCourse, AppError> {
    state.reload_active_manual(epoch).await
}

#[tauri::command]
pub fn course_author_preview(
    markdown: String,
    checks_json: Option<String>,
) -> Result<AuthorPreviewResult, AppError> {
    preview_author_draft(&markdown, checks_json.as_deref())
}

#[tauri::command]
pub async fn course_author_save(
    pack_dir: String,
    markdown: String,
    checks_json: String,
) -> Result<ParsedCourse, AppError> {
    let dir = PathBuf::from(pack_dir);
    tokio::task::spawn_blocking(move || save_author_pack(&dir, &markdown, &checks_json))
        .await
        .map_err(|join_err| {
            AppError::new(ErrorCode::Cancelled, "error.unknown")
                .with_param("detail", join_err.to_string())
                .with_detail(join_err.to_string())
        })?
}
