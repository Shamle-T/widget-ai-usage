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
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
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
    id: String,
    display_name: String,
    enabled: bool,
    detected: bool,
    status: ProviderStatus,
    status_message: Option<String>,
    source: String,
    selected_window_id: Option<String>,
    windows: Vec<UsageWindow>,
    updated_at: Option<DateTime<Utc>>,
    last_successful_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppViewState {
    pub version: u32,
    pub theme: Theme,
    pub weekly_override: bool,
    pub refresh_interval_minutes: u8,
    pub always_on_top: bool,
    pub lock_position: bool,
    pub start_with_system: bool,
    pub refreshing: bool,
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

    pub fn apply_window_preferences(&self, app: &AppHandle) {
        let app = app.clone();
        let settings_path = self.settings_path.clone();
        tauri::async_runtime::spawn(async move {
            let settings = config::load(&settings_path);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(settings.always_on_top);
                if let Some(position) = settings.window_position {
                    // A missing monitor is handled by retaining the centered Tauri default.
                    if position.x >= -16_000
                        && position.y >= -16_000
                        && position.x <= 16_000
                        && position.y <= 16_000
                    {
                        let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
                    }
                }
            }
        });
    }

    pub fn record_window_position(&self, position: PhysicalPosition<i32>) {
        if let Ok(mut values) = self.settings.try_lock() {
            if values.lock_position {
                return;
            }
            values.window_position = Some(position.into());
            let _ = config::save_atomically(&self.settings_path, &values);
        }
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
            weekly_override: settings.weekly_override,
            refresh_interval_minutes: settings.refresh_interval_minutes,
            always_on_top: settings.always_on_top,
            lock_position: settings.lock_position,
            start_with_system: settings.start_with_system,
            refreshing: self.refresh_in_progress.load(Ordering::Relaxed),
            providers,
        }
    }

    pub async fn emit_view(&self, app: &AppHandle) {
        let view = self.view_state().await;
        if let Some(window) = app.get_webview_window("main") {
            let provider_count = view
                .providers
                .iter()
                .filter(|provider| provider.enabled && provider.detected)
                .count()
                .max(1) as f64;
            // Grow for known providers and rely on the frontend scroll fallback past this cap.
            let width = (142.0 + provider_count * 100.0).clamp(350.0, 1_800.0);
            let _ = window.set_size(PhysicalSize::new(width, 156.0));
        }
        let _ = app.emit("usage-state-updated", view);
    }

    pub async fn persist_settings(&self) {
        let settings = self.settings.lock().await;
        let _ = config::save_atomically(&self.settings_path, &settings);
    }

    pub async fn refresh(self: &Arc<Self>, app: AppHandle) {
        if self.refresh_in_progress.swap(true, Ordering::AcqRel) {
            return;
        }
        self.emit_view(&app).await;
        let enabled = self.settings.lock().await.enabled_providers.clone();
        let adapters = self.registry.adapters().to_vec();
        let mut tasks = tokio::task::JoinSet::new();
        for adapter in adapters {
            if enabled.get(adapter.id()).copied().unwrap_or(true) {
                tasks.spawn_blocking(move || {
                    (
                        adapter.id().to_owned(),
                        adapter.source_name().to_owned(),
                        adapter.detect(),
                        adapter.fetch_usage(),
                    )
                });
            }
        }
        while let Some(Ok((id, source, availability, result))) = tasks.join_next().await {
            self.apply_provider_result(id, source, availability, result)
                .await;
        }
        self.refresh_in_progress.store(false, Ordering::Release);
        self.emit_view(&app).await;
    }

    async fn apply_provider_result(
        &self,
        id: String,
        source: String,
        availability: ProviderAvailability,
        result: Result<ProviderUsage, ProviderError>,
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
