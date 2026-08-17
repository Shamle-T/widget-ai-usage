use std::{collections::HashMap, fs, io, path::Path};

use serde::{Deserialize, Serialize};
use tauri::PhysicalPosition;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub theme: Theme,
    pub refresh_interval_minutes: u8,
    pub weekly_override: bool,
    pub always_on_top: bool,
    pub lock_position: bool,
    pub start_with_system: bool,
    pub enabled_providers: HashMap<String, bool>,
    pub provider_order: Vec<String>,
    pub selected_windows: HashMap<String, String>,
    pub window_position: Option<StoredPosition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredPosition {
    pub x: i32,
    pub y: i32,
}

impl From<PhysicalPosition<i32>> for StoredPosition {
    fn from(value: PhysicalPosition<i32>) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl Settings {
    pub fn default_for_app() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            theme: Theme::Dark,
            refresh_interval_minutes: 5,
            weekly_override: false,
            always_on_top: false,
            lock_position: false,
            start_with_system: false,
            enabled_providers: HashMap::from([
                ("codex".to_owned(), true),
                ("claude".to_owned(), true),
                ("kimi".to_owned(), true),
            ]),
            provider_order: vec!["codex".to_owned(), "claude".to_owned(), "kimi".to_owned()],
            selected_windows: HashMap::new(),
            window_position: None,
        }
    }

    pub fn migrate(mut self) -> Self {
        if self.schema_version < SETTINGS_SCHEMA_VERSION {
            self.schema_version = SETTINGS_SCHEMA_VERSION;
        }
        for provider in ["codex", "claude", "kimi"] {
            self.enabled_providers
                .entry(provider.to_owned())
                .or_insert(true);
            if !self.provider_order.iter().any(|value| value == provider) {
                self.provider_order.push(provider.to_owned());
            }
        }
        if !matches!(self.refresh_interval_minutes, 1 | 2 | 5 | 10) {
            self.refresh_interval_minutes = 5;
        }
        self
    }
}

pub fn load(path: &Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<Settings>(&contents).ok())
        .map(Settings::migrate)
        .unwrap_or_else(Settings::default_for_app)
}

pub fn save_atomically(path: &Path, settings: &Settings) -> io::Result<()> {
    let contents = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, contents)?;
    fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::{Settings, SETTINGS_SCHEMA_VERSION};

    #[test]
    fn migration_restores_known_providers_and_valid_interval() {
        let mut settings = Settings::default_for_app();
        settings.schema_version = 0;
        settings.refresh_interval_minutes = 3;
        settings.enabled_providers.clear();
        settings.provider_order.clear();

        let migrated = settings.migrate();
        assert_eq!(migrated.schema_version, SETTINGS_SCHEMA_VERSION);
        assert_eq!(migrated.refresh_interval_minutes, 5);
        assert_eq!(migrated.provider_order, vec!["codex", "claude", "kimi"]);
        assert!(migrated.enabled_providers.values().all(|enabled| *enabled));
    }
}
