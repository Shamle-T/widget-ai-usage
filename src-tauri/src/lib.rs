mod app_state;
mod cache;
mod config;
mod polling;
mod providers;
mod storage;
mod taskbar_widget;
mod tray;

use std::sync::Arc;

use app_state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                let state = state.inner().clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    state.refresh(app).await;
                });
            }
        }))
        .setup(|app| {
            let state = Arc::new(AppState::load(app.handle())?);
            app.manage(state.clone());
            taskbar_widget::initialize(app.handle().clone());
            // The startup refresh creates tray icons once, avoiding competing creation paths.
            polling::start(state, app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("LimitLens failed to start");
}
