use std::{env, path::PathBuf, sync::Arc};

use thiserror::Error;

use super::{claude::ClaudeProvider, codex::CodexProvider, kimi::KimiProvider, ProviderUsage};

#[derive(Debug, Clone)]
pub enum ProviderAvailability {
    Detected,
    NotInstalled,
}

#[derive(Debug, Error, Clone)]
#[allow(dead_code)] // Reserved for future documented adapters; no current adapter fabricates these states.
pub enum ProviderError {
    #[error("Usage metadata is not available through a documented non-interactive interface.")]
    Unsupported,
    #[error("Provider executable was not found.")]
    NotInstalled,
    #[error("Provider response was malformed.")]
    MalformedResponse,
    #[error("Provider authentication is required.")]
    AuthenticationRequired,
    #[error("Unable to refresh provider metadata.")]
    RefreshFailed,
}

pub trait ProviderAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn source_name(&self) -> &'static str;
    fn detect(&self) -> ProviderAvailability;
    fn fetch_usage(&self) -> Result<ProviderUsage, ProviderError>;
}

#[derive(Clone)]
pub struct ProviderRegistry {
    adapters: Vec<Arc<dyn ProviderAdapter>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Arc::new(CodexProvider),
                Arc::new(ClaudeProvider),
                Arc::new(KimiProvider),
            ],
        }
    }

    pub fn adapters(&self) -> &[Arc<dyn ProviderAdapter>] {
        &self.adapters
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapters
            .iter()
            .find(|adapter| adapter.id() == id)
            .cloned()
    }
}

pub(crate) fn executable_exists(name: &str) -> bool {
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| {
            extensions.iter().any(|extension| {
                let mut candidate = PathBuf::from(&directory);
                candidate.push(format!("{name}{extension}"));
                candidate.is_file()
            })
        })
    })
}
