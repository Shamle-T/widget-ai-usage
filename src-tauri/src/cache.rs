use std::{collections::HashMap, fs, io, path::Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{providers::ProviderUsage, storage};

pub const CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageCache {
    pub schema_version: u32,
    pub providers: HashMap<String, CachedProviderUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedProviderUsage {
    pub usage: ProviderUsage,
    pub last_successful_at: DateTime<Utc>,
}

impl UsageCache {
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|contents| serde_json::from_str::<Self>(&contents).ok())
            .filter(|cache| cache.schema_version == CACHE_SCHEMA_VERSION)
            .unwrap_or_else(|| Self {
                schema_version: CACHE_SCHEMA_VERSION,
                providers: HashMap::new(),
            })
    }

    pub fn save_atomically(&self, path: &Path) -> io::Result<()> {
        let contents = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        storage::write_atomically(path, &contents)
    }
}
