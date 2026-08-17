use super::{
    registry::executable_exists, ProviderAdapter, ProviderAvailability, ProviderError,
    ProviderUsage,
};

pub struct ClaudeProvider;

impl ProviderAdapter for ClaudeProvider {
    fn id(&self) -> &'static str {
        "claude"
    }
    fn display_name(&self) -> &'static str {
        "Claude"
    }
    fn source_name(&self) -> &'static str {
        "Claude Code"
    }

    fn detect(&self) -> ProviderAvailability {
        if executable_exists("claude") {
            ProviderAvailability::Detected
        } else {
            ProviderAvailability::NotInstalled
        }
    }

    fn fetch_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Claude Code exposes rate_limits only to a status-line command in an active session.
        // Starting or instrumenting a session would violate LimitLens' no-prompt/no-scraping policy.
        Err(ProviderError::Unsupported)
    }
}
