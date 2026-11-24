use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use codelab_core::{
    App, AppCommand, AppConfig, AppResponse, CleanupHook, DomainError, ShutdownReport,
};
use tauri::{Emitter, Manager, RunEvent, State};

pub const CORE_EVENT_CHANNEL: &str = "codelab://core-event";

pub struct ShellState {
    core: Arc<App>,
}

impl ShellState {
    pub fn open(data_dir: PathBuf) -> Result<Self, DomainError> {
        let core = Arc::new(App::open(AppConfig { data_dir })?);
        Ok(Self { core })
    }

    pub fn dispatch(&self, command: AppCommand) -> Result<AppResponse, DomainError> {
        self.core.execute(command)
    }

    pub fn register_cleanup_hook(&self, name: impl Into<String>, hook: CleanupHook) {
        self.core.register_cleanup_hook(name, hook);
    }

    pub fn on_exit(&self) -> Result<ShutdownReport, DomainError> {
        self.core.shutdown()
    }

    fn subscribe(&self) -> codelab_core::EventSubscription {
        self.core.subscribe()
    }
}

#[tauri::command]
fn dispatch_command(
    state: State<'_, ShellState>,
    command: AppCommand,
) -> Result<AppResponse, DomainError> {
    state.dispatch(command)
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
        .setup(|app| {
            let data_dir = resolve_data_dir(app)?;
            let shell = ShellState::open(data_dir)?;
            let subscription = shell.subscribe();
            app.manage(shell);
            spawn_event_bridge(app.handle().clone(), subscription);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![dispatch_command])
        .build(tauri::generate_context!())
        .expect("failed to build CodeLab Studio desktop shell");

    app.run(|app_handle, event| {
        if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit)
            && let Some(shell) = app_handle.try_state::<ShellState>()
        {
            let _ = shell.on_exit();
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
    use tempfile::TempDir;

    use super::ShellState;

    #[test]
    fn shell_forwards_commands_and_runs_exit_cleanup_hook_idempotently() {
        let dir = TempDir::new().expect("temp dir");
        let shell = ShellState::open(dir.path().to_path_buf()).expect("open shell");

        let cleaned = Arc::new(AtomicUsize::new(0));
        let cleaned_clone = Arc::clone(&cleaned);
        shell.register_cleanup_hook(
            "desktop-smoke-child",
            Box::new(move || {
                cleaned_clone.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        );

        let updated = shell
            .dispatch(AppCommand::UpdateSettings(UpdateSettingsPatch {
                theme: Some(ThemeMode::Dark),
                locale: Some(Locale::EnUs),
                ..Default::default()
            }))
            .expect("dispatch updateSettings");

        match updated {
            AppResponse::Settings(settings) => {
                assert_eq!(settings.theme, ThemeMode::Dark);
                assert_eq!(settings.locale, Locale::EnUs);
            }
            other => panic!("expected Settings, got {other:?}"),
        }

        let first_exit = shell.on_exit().expect("first exit cleanup");
        assert!(!first_exit.already_shut_down);
        assert_eq!(first_exit.hooks_executed, 1);
        assert_eq!(cleaned.load(Ordering::SeqCst), 1);

        let second_exit = shell.on_exit().expect("idempotent second exit cleanup");
        assert!(second_exit.already_shut_down);
        assert_eq!(second_exit.hooks_executed, 0);
        assert_eq!(cleaned.load(Ordering::SeqCst), 1);
    }
}
