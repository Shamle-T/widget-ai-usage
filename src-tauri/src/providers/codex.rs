use super::{
    registry::executable_exists, ProviderAdapter, ProviderAvailability, ProviderError,
    ProviderUsage,
};

pub struct CodexProvider;

impl ProviderAdapter for CodexProvider {
    fn id(&self) -> &'static str {
        "codex"
    }
    fn display_name(&self) -> &'static str {
        "Codex"
    }
    fn source_name(&self) -> &'static str {
        "Codex CLI"
    }

    fn detect(&self) -> ProviderAvailability {
        if executable_exists("codex") {
            ProviderAvailability::Detected
        } else {
            ProviderAvailability::NotInstalled
        }
    }

    fn fetch_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // OpenAI documents /status for an active interactive session, not a headless API.
        // LimitLens refuses to drive a terminal UI or create a model turn to infer usage.
        Err(ProviderError::Unsupported)
    }
}
