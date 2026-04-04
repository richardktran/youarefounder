//! Ollama adapter — calls the local Ollama `/api/chat` endpoint.
//!
//! Ollama API reference: https://github.com/ollama/ollama/blob/main/docs/api.md

use ai_core::{AiError, ChatCompletionRequest, ChatCompletionResponse, InferenceProvider, Role};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};

// ─── Wire types ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OllamaMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaResponseMessage,
    done: bool,
    done_reason: Option<String>,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

// ─── Error body returned by Ollama on 4xx/5xx ────────────────────────────────

#[derive(Deserialize)]
struct OllamaErrorBody {
    error: String,
}

// ─── Adapter ──────────────────────────────────────────────────────────────────

pub struct OllamaAdapter {
    base_url: String,
    client: reqwest::Client,
}

/// Default cap for how long a single `/api/chat` call may run (inference + body read).
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 600;

/// Fail TCP connect quickly when Ollama is not listening.
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

impl OllamaAdapter {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_timeouts(base_url, DEFAULT_REQUEST_TIMEOUT_SECS, DEFAULT_CONNECT_TIMEOUT_SECS)
    }

    /// `request_timeout_secs` bounds the entire chat request (including model inference).
    pub fn with_timeouts(
        base_url: impl Into<String>,
        request_timeout_secs: u64,
        connect_timeout_secs: u64,
    ) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(connect_timeout_secs))
            .timeout(Duration::from_secs(request_timeout_secs))
            .build()
            .expect("reqwest client with timeouts");

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        }
    }
}

#[async_trait]
impl InferenceProvider for OllamaAdapter {
    fn kind(&self) -> &str {
        "ollama"
    }

    async fn complete(
        &self,
        req: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, AiError> {
        let messages: Vec<OllamaMessage> = req
            .messages
            .into_iter()
            .map(|m| OllamaMessage {
                role: role_str(&m.role),
                content: m.content,
            })
            .collect();

        let options = if req.temperature.is_some() || req.max_tokens.is_some() {
            Some(OllamaOptions {
                temperature: req.temperature,
                num_predict: req.max_tokens,
            })
        } else {
            None
        };

        let body = OllamaChatRequest {
            model: req.model,
            messages,
            stream: false,
            options,
        };

        let url = format!("{}/api/chat", self.base_url);
        debug!(url, "ollama chat request");

        info!(
            model = %body.model,
            url = %url,
            "Ollama: sending chat request (first reply can take a long time while the model loads or on CPU)"
        );

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("timed out") || msg.contains("timeout") {
                    AiError::RequestFailed(format!(
                        "ollama request timed out (increase `request_timeout_secs` in AI profile config if needed): {msg}"
                    ))
                } else {
                    AiError::ConnectionFailed(msg)
                }
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            // Try to surface Ollama's own error message; fall back to raw body.
            let body_bytes = resp.bytes().await.unwrap_or_default();
            let reason = serde_json::from_slice::<OllamaErrorBody>(&body_bytes)
                .map(|e| e.error)
                .unwrap_or_else(|_| String::from_utf8_lossy(&body_bytes).into_owned());
            return Err(AiError::RequestFailed(format!(
                "ollama {status}: {reason}"
            )));
        }

        let parsed: OllamaChatResponse = resp
            .json()
            .await
            .map_err(|e| AiError::RequestFailed(format!("failed to parse response: {e}")))?;

        Ok(ChatCompletionResponse {
            content: parsed.message.content,
            model: parsed.model,
            finish_reason: if parsed.done { parsed.done_reason } else { None },
        })
    }

    async fn health_check(&self) -> Result<(), AiError> {
        let url = format!("{}/api/tags", self.base_url);
        debug!(url, "ollama health check");

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiError::ConnectionFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(AiError::ConnectionFailed(format!(
                "ollama health check returned {}",
                resp.status()
            )))
        }
    }
}

fn role_str(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}
