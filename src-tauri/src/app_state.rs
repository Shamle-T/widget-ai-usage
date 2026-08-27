use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::Mutex;

use crate::{
    cache::{CachedProviderUsage, UsageCache},
    config::{self, Settings, Theme},
    providers::{
        normalize_used_percent, ProviderAvailability, ProviderError, ProviderRegistry,
        ProviderUsage, UsageWindow,
    },
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Loading,
    Detected,
    Live,
    Cached,
    Stale,
    AuthenticationRequired,
    NotInstalled,
    Unsupported,
    Error,
}

#[derive(Debug, Clone)]
pub(crate) struct ProviderRuntime {
    pub(crate) detected: bool,
    pub(crate) status: ProviderStatus,
    pub(crate) status_message: Option<String>,
    pub(crate) source: String,
    pub(crate) usage: Option<ProviderUsage>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
    pub(crate) last_successful_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) enabled: bool,
    pub(crate) detected: bool,
    pub(crate) status: ProviderStatus,
    pub(crate) status_message: Option<String>,
    pub(crate) source: String,
    pub(crate) selected_window_id: Option<String>,
    pub(crate) windows: Vec<UsageWindow>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
    pub(crate) last_successful_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppViewState {
    pub version: u32,
    pub theme: Theme,
    pub refresh_interval_minutes: u8,
    pub start_with_system: bool,
    pub refreshing: bool,
    pub taskbar_offset: Option<i32>,
    pub providers: Vec<ProviderView>,
}

pub struct AppState {
    settings_path: PathBuf,
    cache_path: PathBuf,
    pub(crate) settings: Mutex<Settings>,
    cache: Mutex<UsageCache>,
    pub(crate) runtime: Mutex<HashMap<String, ProviderRuntime>>,
    pub registry: ProviderRegistry,
    pub refresh_in_progress: AtomicBool,
}

impl AppState {
    pub fn load(app: &AppHandle) -> tauri::Result<Self> {
        let data_dir = app.path().app_data_dir()?;
        fs::create_dir_all(&data_dir)?;
        let settings_path = data_dir.join("settings.json");
        let cache_path = data_dir.join("usage-cache.json");
        let settings = config::load(&settings_path);
        let cache = UsageCache::load(&cache_path);
        let registry = ProviderRegistry::new();
        let runtime = registry
            .adapters()
            .iter()
            .map(|adapter| {
                let cached = cache.providers.get(adapter.id());
                (
                    adapter.id().to_owned(),
                    ProviderRuntime {
                        detected: false,
                        status: if cached.is_some() {
                            ProviderStatus::Cached
                        } else {
                            ProviderStatus::Loading
                        },
                        status_message: None,
                        source: adapter.source_name().to_owned(),
                        usage: cached.map(|entry| entry.usage.clone()),
                        updated_at: cached.map(|entry| entry.usage.updated_at),
                        last_successful_at: cached.map(|entry| entry.last_successful_at),
                    },
                )
            })
            .collect();
        Ok(Self {
            settings_path,
            cache_path,
            settings: Mutex::new(settings),
            cache: Mutex::new(cache),
            runtime: Mutex::new(runtime),
            registry,
            refresh_in_progress: AtomicBool::new(false),
        })
    }

    pub async fn view_state(&self) -> AppViewState {
        let settings = self.settings.lock().await.clone();
        let runtime = self.runtime.lock().await;
        let mut providers = Vec::new();
        for id in &settings.provider_order {
            if let (Some(adapter), Some(current)) = (self.registry.get(id), runtime.get(id)) {
                let windows = current
                    .usage
                    .as_ref()
                    .map(|usage| usage.windows.clone())
                    .unwrap_or_default();
                let saved = settings.selected_windows.get(id).cloned();
                let selected = saved
                    .filter(|selection| windows.iter().any(|window| &window.id == selection))
                    .or_else(|| windows.first().map(|window| window.id.clone()));
                providers.push(ProviderView {
                    id: id.clone(),
                    display_name: adapter.display_name().to_owned(),
                    enabled: settings.enabled_providers.get(id).copied().unwrap_or(true),
                    detected: current.detected,
                    status: current.status.clone(),
                    status_message: current.status_message.clone(),
                    source: current.source.clone(),
                    selected_window_id: selected,
                    windows,
                    updated_at: current.updated_at,
                    last_successful_at: current.last_successful_at,
                });
            }
        }
        AppViewState {
            version: 1,
            theme: settings.theme,
            refresh_interval_minutes: settings.refresh_interval_minutes,
            start_with_system: settings.start_with_system,
            refreshing: self.refresh_in_progress.load(Ordering::Relaxed),
            taskbar_offset: settings.taskbar_offset,
            providers,
        }
    }

    pub async fn sync_tray(&self, app: &AppHandle) {
        let view = self.view_state().await;
        let _ = crate::tray::sync(app, &view);
        crate::taskbar_widget::sync(app, &view);
    }

    pub async fn persist_settings(&self) {
        let settings = self.settings.lock().await;
        let _ = config::save_atomically(&self.settings_path, &settings);
    }

    pub async fn refresh(self: &Arc<Self>, app: AppHandle) {
        if self.refresh_in_progress.swap(true, Ordering::AcqRel) {
            return;
        }
        self.sync_tray(&app).await;
        let enabled = self.settings.lock().await.enabled_providers.clone();
        let adapters = self.registry.adapters().to_vec();
        let mut tasks = tokio::task::JoinSet::new();
        for adapter in adapters {
            let provider_enabled = enabled.get(adapter.id()).copied().unwrap_or(true);
            tasks.spawn_blocking(move || {
                let availability = adapter.detect();
                let result = if provider_enabled
                    && matches!(&availability, ProviderAvailability::Detected)
                {
                    Some(adapter.fetch_usage())
                } else {
                    None
                };
                (
                    adapter.id().to_owned(),
                    adapter.source_name().to_owned(),
                    availability,
                    result,
                )
            });
        }
        while let Some(Ok((id, source, availability, result))) = tasks.join_next().await {
            self.apply_provider_result(id, source, availability, result)
                .await;
        }
        self.refresh_in_progress.store(false, Ordering::Release);
        self.sync_tray(&app).await;
    }

    pub async fn set_provider_enabled(
        &self,
        app: &AppHandle,
        provider_id: &str,
        enabled: bool,
    ) -> Result<(), String> {
        if self.registry.get(provider_id).is_none() {
            return Err("Unknown provider.".to_owned());
        }
        self.settings
            .lock()
            .await
            .enabled_providers
            .insert(provider_id.to_owned(), enabled);
        self.persist_settings().await;
        self.sync_tray(app).await;
        Ok(())
    }

    pub async fn set_refresh_interval(&self, app: &AppHandle, minutes: u8) -> Result<(), String> {
        if !matches!(minutes, 1 | 2 | 5 | 10) {
            return Err("Refresh interval must be 1, 2, 5, or 10 minutes.".to_owned());
        }
        self.settings.lock().await.refresh_interval_minutes = minutes;
        self.persist_settings().await;
        self.sync_tray(app).await;
        Ok(())
    }

    pub async fn set_theme(&self, app: &AppHandle, theme: Theme) {
        self.settings.lock().await.theme = theme;
        self.persist_settings().await;
        self.sync_tray(app).await;
    }

    pub async fn set_taskbar_offset(&self, offset: i32) {
        self.settings.lock().await.taskbar_offset = Some(offset);
        self.persist_settings().await;
    }

    pub async fn cycle_provider_window(
        &self,
        app: &AppHandle,
        provider_id: &str,
    ) -> Result<(), String> {
        let windows = self
            .runtime
            .lock()
            .await
            .get(provider_id)
            .and_then(|current| current.usage.as_ref().map(|usage| usage.windows.clone()))
            .unwrap_or_default();
        if windows.len() < 2 {
            return Ok(());
        }

        let mut settings = self.settings.lock().await;
        let next_index = next_window_index(
            &windows,
            settings
                .selected_windows
                .get(provider_id)
                .map(String::as_str),
        );
        settings
            .selected_windows
            .insert(provider_id.to_owned(), windows[next_index].id.clone());
        drop(settings);
        self.persist_settings().await;
        self.sync_tray(app).await;
        Ok(())
    }

    async fn apply_provider_result(
        &self,
        id: String,
        source: String,
        availability: ProviderAvailability,
        result: Option<Result<ProviderUsage, ProviderError>>,
    ) {
        let mut runtime = self.runtime.lock().await;
        let Some(current) = runtime.get_mut(&id) else {
            return;
        };
        current.source = source;
        current.detected = matches!(availability, ProviderAvailability::Detected);
        if matches!(availability, ProviderAvailability::NotInstalled) {
            current.status = ProviderStatus::NotInstalled;
            current.status_message = Some("Provider client was not detected on PATH.".to_owned());
            return;
        }
        let Some(result) = result else {
            if current.usage.is_some() {
                current.status = ProviderStatus::Cached;
            } else {
                current.status = ProviderStatus::Detected;
                current.status_message = Some("Provider client was detected on PATH.".to_owned());
            }
            return;
        };
        match result {
            Ok(mut usage) => {
                for window in &mut usage.windows {
                    window.used_percent = normalize_used_percent(window.used_percent);
                }
                let successful_at = Utc::now();
                current.updated_at = Some(usage.updated_at);
                current.last_successful_at = Some(successful_at);
                current.usage = Some(usage.clone());
                current.status = ProviderStatus::Live;
                current.status_message = None;
                drop(runtime);
                let mut cache = self.cache.lock().await;
                cache.providers.insert(
                    id,
                    CachedProviderUsage {
                        usage,
                        last_successful_at: successful_at,
                    },
                );
                let _ = cache.save_atomically(&self.cache_path);
            }
            Err(error) => {
                current.status = match error {
                    ProviderError::Unsupported => ProviderStatus::Unsupported,
                    ProviderError::AuthenticationRequired => ProviderStatus::AuthenticationRequired,
                    ProviderError::NotInstalled => ProviderStatus::NotInstalled,
                    _ if current.usage.is_some() => ProviderStatus::Stale,
                    _ => ProviderStatus::Error,
                };
                current.status_message = Some(error.to_string());
            }
        }
    }

    pub async fn set_autostart(&self, app: &AppHandle, enabled: bool) -> Result<(), String> {
        let manager = app.autolaunch();
        if enabled {
            manager
                .enable()
                .map_err(|_| "Unable to enable system startup.".to_owned())?;
        } else {
            manager
                .disable()
                .map_err(|_| "Unable to disable system startup.".to_owned())?;
        }
        self.settings.lock().await.start_with_system = enabled;
        self.persist_settings().await;
        Ok(())
    }
}

fn next_window_index(windows: &[UsageWindow], selected: Option<&str>) -> usize {
    if windows.is_empty() {
        return 0;
    }
    selected
        .and_then(|selected| windows.iter().position(|window| window.id == selected))
        .map(|index| (index + 1) % windows.len())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::next_window_index;
    use crate::providers::{UsageWindow, UsageWindowKind};

    fn window(id: &str) -> UsageWindow {
        UsageWindow {
            id: id.to_owned(),
            display_name: id.to_owned(),
            kind: UsageWindowKind::ProviderSpecific,
            used_percent: Some(0.0),
            resets_at: None,
        }
    }

    #[test]
    fn cycles_only_through_the_selected_providers_windows() {
        let windows = vec![window("hourly"), window("weekly")];
        assert_eq!(next_window_index(&windows, Some("hourly")), 1);
        assert_eq!(next_window_index(&windows, Some("weekly")), 0);
        assert_eq!(next_window_index(&windows, Some("removed-window")), 0);
    }
}
