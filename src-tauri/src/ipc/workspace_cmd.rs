use crate::domain::error::AppError;
use crate::domain::ids::SessionEpoch;
use crate::session::AppState;

#[tauri::command]
pub async fn workspace_close(
    epoch: SessionEpoch,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
    let _ = state.close_workspace(epoch).await?;
    Ok(())
}
