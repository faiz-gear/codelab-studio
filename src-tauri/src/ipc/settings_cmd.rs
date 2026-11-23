use crate::domain::error::AppError;
use crate::domain::ids::SessionEpoch;
use crate::domain::metrics::MetricEvent;
use crate::domain::workspace::{GlobalConfigPatch, GlobalConfigView};
use crate::session::AppState;

#[tauri::command]
pub fn settings_update(
    patch: GlobalConfigPatch,
    state: tauri::State<'_, AppState>,
) -> Result<GlobalConfigView, AppError> {
    state.update_settings(patch)
}

#[tauri::command]
pub async fn metrics_record(
    epoch: Option<SessionEpoch>,
    event: MetricEvent,
    state: tauri::State<'_, AppState>,
) -> Result<(), AppError> {
    state.record_metric(epoch, event).await
}
