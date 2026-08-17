use std::sync::Arc;

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::{app_state::AppState, config::Theme, main_window};

#[tauri::command]
pub async fn get_view_state(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::app_state::AppViewState, String> {
    Ok(state.view_state().await)
}

#[tauri::command]
pub async fn refresh_now(state: State<'_, Arc<AppState>>, app: AppHandle) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        state.refresh(app).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn set_provider_enabled(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    provider_id: String,
    enabled: bool,
) -> Result<(), String> {
    if state.registry.get(&provider_id).is_none() {
        return Err("Unknown provider.".to_owned());
    }
    state
        .settings
        .lock()
        .await
        .enabled_providers
        .insert(provider_id, enabled);
    state.persist_settings().await;
    state.emit_view(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn set_refresh_interval(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    minutes: u8,
) -> Result<(), String> {
    if !matches!(minutes, 1 | 2 | 5 | 10) {
        return Err("Refresh interval must be 1, 2, 5, or 10 minutes.".to_owned());
    }
    state.settings.lock().await.refresh_interval_minutes = minutes;
    state.persist_settings().await;
    state.emit_view(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn set_theme(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    theme: Theme,
) -> Result<(), String> {
    state.settings.lock().await.theme = theme;
    state.persist_settings().await;
    state.emit_view(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn set_weekly_override(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    state.settings.lock().await.weekly_override = enabled;
    state.persist_settings().await;
    state.emit_view(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn cycle_provider_window(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    provider_id: String,
) -> Result<(), String> {
    let windows = state
        .runtime
        .lock()
        .await
        .get(&provider_id)
        .and_then(|current| current.usage.as_ref().map(|usage| usage.windows.clone()))
        .unwrap_or_default();
    if windows.len() < 2 {
        return Ok(());
    }
    let mut settings = state.settings.lock().await;
    if settings.weekly_override {
        return Ok(());
    }
    let current = settings.selected_windows.get(&provider_id).cloned();
    let index = windows
        .iter()
        .position(|window| Some(&window.id) == current.as_ref())
        .unwrap_or(usize::MAX);
    let next = &windows[(index + 1) % windows.len()];
    settings
        .selected_windows
        .insert(provider_id, next.id.clone());
    drop(settings);
    state.persist_settings().await;
    state.emit_view(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn set_always_on_top(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    if let Some(window) = main_window(&app) {
        let _ = window.set_always_on_top(enabled);
    }
    state.settings.lock().await.always_on_top = enabled;
    state.persist_settings().await;
    state.emit_view(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn set_lock_position(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    state.settings.lock().await.lock_position = enabled;
    state.persist_settings().await;
    state.emit_view(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn set_start_with_system(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    state.set_autostart(&app, enabled).await?;
    state.emit_view(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn test_connection(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    provider_id: String,
) -> Result<(), String> {
    let provider = state
        .registry
        .get(&provider_id)
        .ok_or_else(|| "Unknown provider.".to_owned())?;
    let state = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        state.refresh(app).await;
    });
    if matches!(
        provider.detect(),
        crate::providers::ProviderAvailability::NotInstalled
    ) {
        return Err("Provider client was not detected on PATH.".to_owned());
    }
    Err("Usage metadata is unavailable because this provider has no documented non-interactive quota query.".to_owned())
}

#[tauri::command]
pub fn hide_widget(app: AppHandle) {
    if let Some(window) = main_window(&app) {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn quit_application(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    open_utility_window(
        &app,
        "settings",
        "LimitLens Settings",
        "index.html?settings=1",
        520.0,
        560.0,
    )
}

#[tauri::command]
pub fn open_about(app: AppHandle) -> Result<(), String> {
    open_utility_window(
        &app,
        "settings",
        "About LimitLens",
        "index.html?about=1",
        460.0,
        420.0,
    )
}

fn open_utility_window(
    app: &AppHandle,
    label: &str,
    title: &str,
    url: &str,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        window
            .show()
            .map_err(|_| "Unable to show settings.".to_owned())?;
        window
            .set_focus()
            .map_err(|_| "Unable to focus settings.".to_owned())?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(width, height)
        .min_inner_size(400.0, 360.0)
        .resizable(true)
        .build()
        .map(|_| ())
        .map_err(|_| "Unable to open settings.".to_owned())
}
