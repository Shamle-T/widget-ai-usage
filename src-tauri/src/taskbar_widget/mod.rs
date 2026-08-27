#[cfg(target_os = "windows")]
mod windows;

use tauri::AppHandle;

use crate::app_state::AppViewState;

#[cfg(target_os = "windows")]
pub fn initialize(app: AppHandle) {
    windows::initialize(app);
}

#[cfg(not(target_os = "windows"))]
pub fn initialize(_app: AppHandle) {}

#[cfg(target_os = "windows")]
pub fn sync(app: &AppHandle, view: &AppViewState) {
    windows::sync(app, view);
}

#[cfg(not(target_os = "windows"))]
pub fn sync(_app: &AppHandle, _view: &AppViewState) {}
