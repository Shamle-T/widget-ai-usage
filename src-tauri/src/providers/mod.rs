mod claude;
mod codex;
mod kimi;
mod registry;

pub use registry::{ProviderAdapter, ProviderAvailability, ProviderError, ProviderRegistry};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageWindowKind {
    ShortTerm,
    Daily,
    Weekly,
    Monthly,
    ProviderSpecific,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub id: String,
    pub display_name: String,
    pub kind: UsageWindowKind,
    pub used_percent: Option<f64>,
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub provider_id: String,
    pub provider_name: String,
    pub account_label: Option<String>,
    pub windows: Vec<UsageWindow>,
    pub updated_at: DateTime<Utc>,
    pub source: String,
}

pub fn normalize_used_percent(value: Option<f64>) -> Option<f64> {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::normalize_used_percent;

    #[test]
    fn normalizes_external_percentages_without_inverting_semantics() {
        assert_eq!(normalize_used_percent(Some(24.5)), Some(24.5));
        assert_eq!(normalize_used_percent(Some(-1.0)), Some(0.0));
        assert_eq!(normalize_used_percent(Some(101.0)), Some(100.0));
        assert_eq!(normalize_used_percent(None), None);
    }

    #[test]
    fn rejects_non_finite_provider_values() {
        assert_eq!(normalize_used_percent(Some(f64::NAN)), None);
        assert_eq!(normalize_used_percent(Some(f64::INFINITY)), None);
    }
}
