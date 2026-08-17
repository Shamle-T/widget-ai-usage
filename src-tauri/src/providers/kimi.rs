use super::{
    registry::executable_exists, ProviderAdapter, ProviderAvailability, ProviderError,
    ProviderUsage,
};

pub struct KimiProvider;

impl ProviderAdapter for KimiProvider {
    fn id(&self) -> &'static str {
        "kimi"
    }
    fn display_name(&self) -> &'static str {
        "Kimi"
    }
    fn source_name(&self) -> &'static str {
        "Kimi Code"
    }

    fn detect(&self) -> ProviderAvailability {
        if executable_exists("kimi") {
            ProviderAvailability::Detected
        } else {
            ProviderAvailability::NotInstalled
        }
    }

    fn fetch_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Kimi documents an interactive /usage command, but no stable external quota protocol.
        Err(ProviderError::Unsupported)
    }
}
