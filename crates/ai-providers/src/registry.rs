//! Provider registry — resolves `provider_kind` slugs to adapter instances.
//!
//! Phase 1: only `ollama` is enabled. Adding a new vendor = implement adapter
//! + add an arm to `build_adapter` + add to `enabled_providers`.

use ai_core::{AiError, InferenceProvider};
use serde::{Deserialize, Serialize};

use crate::ollama::{OllamaAdapter, DEFAULT_REQUEST_TIMEOUT_SECS};

// ─── Provider metadata (returned by GET /v1/ai-providers) ────────────────────

/// Describes one enabled provider for the frontend provider picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Stable slug stored in `ai_profiles.provider_kind`.
    pub kind: String,
    pub display_name: String,
    /// JSON Schema–like descriptor for fields the UI must collect.
    pub config_fields: Vec<ConfigField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Url,
    Password,
}

// ─── Registry ─────────────────────────────────────────────────────────────────

/// Resolves `provider_kind` → `Box<dyn InferenceProvider>` on demand.
///
/// Cloneable so it can live in `AppState`.
#[derive(Clone, Debug, Default)]
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn new() -> Self {
        Self
    }

    /// Returns metadata for all providers enabled in this build.
    /// Phase 1: only Ollama. Add more rows here in later phases.
    pub fn enabled_providers(&self) -> Vec<ProviderInfo> {
        vec![ProviderInfo {
            kind: "ollama".into(),
            display_name: "Ollama (Local)".into(),
            config_fields: vec![ConfigField {
                key: "base_url".into(),
                label: "Base URL".into(),
                field_type: FieldType::Url,
                required: true,
                default_value: Some("http://127.0.0.1:11434".into()),
                placeholder: Some("http://127.0.0.1:11434".into()),
            }],
        }]
    }

    /// Builds an adapter from `provider_kind` + the `provider_config` JSONB.
    /// Returns `Err(AiError::UnsupportedProvider)` for unknown/disabled kinds.
    pub fn build_adapter(
        &self,
        provider_kind: &str,
        provider_config: &serde_json::Value,
    ) -> Result<Box<dyn InferenceProvider>, AiError> {
        match provider_kind {
            "ollama" => {
                let base_url = provider_config
                    .get("base_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("http://127.0.0.1:11434")
                    .to_string();
                let request_timeout_secs = provider_config
                    .get("request_timeout_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS)
                    .clamp(30, 7200);
                Ok(Box::new(OllamaAdapter::with_timeouts(
                    base_url,
                    request_timeout_secs,
                    10,
                )))
            }
            kind => Err(AiError::UnsupportedProvider(kind.to_string())),
        }
    }
}
