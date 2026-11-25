use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use codelab_core::{App, AppCommand, AppConfig, AppResponse, DomainError};
use serde::Deserialize;
use tauri::{Emitter, Manager, RunEvent, State};
use tauri_plugin_dialog::DialogExt;

pub const CORE_EVENT_CHANNEL: &str = "codelab://core-event";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum LocalPathPickKind {
    MarkdownFile,
    CoursePackDir,
    WorkspaceDir,
}

#[tauri::command(async)]
fn dispatch_command(
    core: State<'_, Arc<App>>,
    command: AppCommand,
) -> Result<AppResponse, DomainError> {
    core.execute(command)
}

#[tauri::command]
async fn pick_local_path(app: tauri::AppHandle, kind: LocalPathPickKind) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = match kind {
            LocalPathPickKind::CoursePackDir | LocalPathPickKind::WorkspaceDir => {
                app.dialog().file().blocking_pick_folder()
            }
            LocalPathPickKind::MarkdownFile => app
                .dialog()
                .file()
                .add_filter("Markdown", &["md", "markdown"])
                .blocking_pick_file(),
        };
        picked
            .and_then(|file_path| file_path.into_path().ok())
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

fn open_core(data_dir: PathBuf) -> Result<Arc<App>, DomainError> {
    Ok(Arc::new(App::open(AppConfig { data_dir })?))
}

fn handle_run_event(core: &App, event: &RunEvent) {
    if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
        let _ = core.shutdown();
    }
}

fn resolve_data_dir(app: &tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(override_dir) = std::env::var("CODELAB_DATA_DIR") {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    Ok(app.path().app_data_dir()?)
}

fn spawn_event_bridge(app_handle: tauri::AppHandle, subscription: codelab_core::EventSubscription) {
    let _ = thread::Builder::new()
        .name("codelab-core-event-bridge".into())
        .spawn(move || {
            while let Ok(event) = subscription.recv() {
                if app_handle.emit(CORE_EVENT_CHANNEL, &event).is_err() {
                    break;
                }
            }
        });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = resolve_data_dir(app)?;
            let core = open_core(data_dir)?;
            let subscription = core.subscribe();
            app.manage(core);
            spawn_event_bridge(app.handle().clone(), subscription);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![dispatch_command, pick_local_path])
        .build(tauri::generate_context!())
        .expect("failed to build CodeLab Studio desktop shell");

    app.run(|app_handle, event| {
        if let Some(core) = app_handle.try_state::<Arc<App>>() {
            handle_run_event(&core, &event);
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use codelab_core::{AppCommand, AppResponse, Locale, ThemeMode, UpdateSettingsPatch};
    use tauri::RunEvent;
    use tempfile::TempDir;

    use super::{handle_run_event, open_core};

    #[test]
    fn shell_manages_core_and_exit_run_event_triggers_cleanup_idempotently() {
        let dir = TempDir::new().expect("temp dir");
        let core = open_core(dir.path().to_path_buf()).expect("open core");

        let cleaned = Arc::new(AtomicUsize::new(0));
        let cleaned_clone = Arc::clone(&cleaned);
        core.register_cleanup_hook(
            "desktop-smoke-child",
            Box::new(move || {
                cleaned_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        );

        let updated = core
            .execute(AppCommand::UpdateSettings(UpdateSettingsPatch {
                theme: Some(ThemeMode::Dark),
                locale: Some(Locale::EnUs),
                ..Default::default()
            }))
            .expect("execute updateSettings");

        match updated {
            AppResponse::Settings(settings) => {
                assert_eq!(settings.theme, ThemeMode::Dark);
                assert_eq!(settings.locale, Locale::EnUs);
            }
            other => panic!("expected Settings, got {other:?}"),
        }

        handle_run_event(&core, &RunEvent::Exit);
        assert_eq!(cleaned.load(Ordering::SeqCst), 1);

        handle_run_event(&core, &RunEvent::Exit);
        assert_eq!(cleaned.load(Ordering::SeqCst), 1);
    }
}
