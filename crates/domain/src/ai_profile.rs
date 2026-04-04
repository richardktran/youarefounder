use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stored AI provider configuration linked to a person (co-founder, CEO, etc.).
///
/// `provider_kind` is a stable slug (`ollama`, `openai_api`, …).
/// `provider_config` is JSONB carrying provider-specific fields (e.g. base_url for Ollama).
/// API keys / secrets are stored separately and never returned to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProfile {
    pub id: Uuid,
    pub company_id: Uuid,
    pub display_name: Option<String>,
    /// Stable slug: `ollama` | `openai_api` | `anthropic` | `google_gemini` | `azure_openai`
    pub provider_kind: String,
    /// Vendor-opaque model identifier (e.g. `llama3.2`, `gpt-4o`).
    pub model_id: String,
    /// Provider-specific config fields (schema_version inside for forward compat).
    pub provider_config: serde_json::Value,
    pub default_temperature: Option<f64>,
    pub default_max_tokens: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAiProfileInput {
    pub display_name: Option<String>,
    pub provider_kind: String,
    pub model_id: String,
    pub provider_config: Option<serde_json::Value>,
    pub default_temperature: Option<f64>,
    pub default_max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAiProfileInput {
    pub display_name: Option<String>,
    pub model_id: Option<String>,
    pub provider_config: Option<serde_json::Value>,
    pub default_temperature: Option<f64>,
    pub default_max_tokens: Option<i32>,
}
