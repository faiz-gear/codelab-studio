pub mod domain;
pub mod ipc;
pub mod platform;
pub mod session;
pub mod storage;

use std::io::Write;

use tauri::Manager;

use crate::session::AppState;

pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_min_size(Some(tauri::LogicalSize::new(1024.0, 720.0)));
                if std::env::var("CODELAB_STARTUP_PROBE").as_deref() == Ok("1") {
                    let scale = window.scale_factor().unwrap_or(1.0);
                    if let Ok(initial) = window.inner_size() {
                        let initial_logical = initial.to_logical::<f64>(scale);
                        println!(
                            "CODELAB_WINDOW_INITIAL {{\"width\":{:.0},\"height\":{:.0},\"minWidth\":1024,\"minHeight\":720,\"scale\":{:.2}}}",
                            initial_logical.width, initial_logical.height, scale
                        );
                    }
                    let _ = std::io::stdout().flush();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::bootstrap_cmd::app_bootstrap,
            ipc::settings_cmd::settings_update,
            ipc::settings_cmd::metrics_record,
            ipc::workspace_cmd::workspace_close,
        ])
        .build(tauri::generate_context!())
        .expect("error while building CodeLab Studio")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let _ = app_handle
                    .state::<AppState>()
                    .drain_active_session_for_shutdown();
            }
        });
}
