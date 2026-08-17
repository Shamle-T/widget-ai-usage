mod app_state;
mod cache;
mod commands;
mod config;
mod polling;
mod providers;
mod tray;

use std::sync::Arc;

use app_state::AppState;
use tauri::{Manager, WebviewWindow, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_widget(app);
        }))
        .setup(|app| {
            let state = Arc::new(AppState::load(app.handle())?);
            app.manage(state.clone());
            state.apply_window_preferences(app.handle());
            tray::create(app.handle())?;
            polling::start(state, app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Moved(position) if window.label() == "main" => {
                if let Some(state) = window.app_handle().try_state::<Arc<AppState>>() {
                    state.record_window_position(*position);
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_view_state,
            commands::refresh_now,
            commands::set_provider_enabled,
            commands::set_refresh_interval,
            commands::set_theme,
            commands::set_weekly_override,
            commands::cycle_provider_window,
            commands::set_always_on_top,
            commands::set_lock_position,
            commands::set_start_with_system,
            commands::test_connection,
            commands::hide_widget,
            commands::quit_application,
            commands::open_settings,
            commands::open_about
        ])
        .run(tauri::generate_context!())
        .expect("LimitLens failed to start");
}

pub fn show_widget(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn main_window(app: &tauri::AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}
