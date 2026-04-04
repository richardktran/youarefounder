//! Stable internal contract for LLM inference.
//!
//! All provider adapters translate to/from these types.
//! The worker and API never import vendor SDKs directly.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ─── Message ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into() }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into() }
    }
}

// ─── Request / Response ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub content: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

// ─── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("request failed: {0}")]
    RequestFailed(String),

    #[error("unsupported provider: {0}")]
    UnsupportedProvider(String),

    #[error("configuration error: {0}")]
    Configuration(String),
}

// ─── Trait ────────────────────────────────────────────────────────────────────

/// Implemented by each vendor adapter (Ollama, OpenAI, …).
///
/// The registry resolves the right adapter from `AIProfile.provider_kind` and
/// dispatches calls here. Worker code only sees this trait.
#[async_trait]
pub trait InferenceProvider: Send + Sync {
    /// Stable slug matching `AIProfile.provider_kind` (e.g. `"ollama"`).
    fn kind(&self) -> &str;

    /// Run one turn of inference. Waits for the full response.
    async fn complete(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError>;

    /// Lightweight liveness check — used by the "Test connection" UI button.
    async fn health_check(&self) -> Result<(), AiError>;
}
