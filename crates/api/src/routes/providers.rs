use ai_core::{ChatCompletionRequest, Message};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{error::ApiResult, state::AppState};

/// `GET /v1/ai-providers`
///
/// Lists AI providers enabled in this build and their required config fields.
/// The frontend uses this to render the provider picker dynamically.
/// Phase 1: only `ollama` is returned.
pub async fn list_providers(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let providers = state.providers.enabled_providers();
    Ok(Json(serde_json::json!({ "providers": providers })))
}

// ─── Test-connection (unauthenticated / pre-profile) ──────────────────────────

#[derive(Debug, Deserialize)]
pub struct TestConnectionInput {
    pub provider_kind: String,
    pub provider_config: serde_json::Value,
    /// When provided, a minimal chat request is sent to verify the model is
    /// available and responding — not just that Ollama is reachable.
    pub model_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResult {
    pub ok: bool,
    pub error: Option<String>,
}

/// `POST /v1/ai-providers/test-connection`
///
/// Validates the given provider config by running a lightweight health check.
/// Called from the onboarding AI step before an `AIProfile` row is created.
pub async fn test_connection(
    State(state): State<AppState>,
    Json(input): Json<TestConnectionInput>,
) -> ApiResult<Json<TestConnectionResult>> {
    let adapter = match state
        .providers
        .build_adapter(&input.provider_kind, &input.provider_config)
    {
        Ok(a) => a,
        Err(e) => {
            return Ok(Json(TestConnectionResult {
                ok: false,
                error: Some(e.to_string()),
            }));
        }
    };

    // Step 1: verify Ollama is reachable.
    if let Err(e) = adapter.health_check().await {
        return Ok(Json(TestConnectionResult {
            ok: false,
            error: Some(e.to_string()),
        }));
    }

    // Step 2: if a model was specified, send a minimal chat request to confirm
    // the model is loaded and responding — catches "model not found" errors.
    if let Some(model_id) = input.model_id {
        let req = ChatCompletionRequest {
            model: model_id,
            messages: vec![Message::user("hi")],
            temperature: None,
            max_tokens: Some(16),
        };
        if let Err(e) = adapter.complete(req).await {
            return Ok(Json(TestConnectionResult {
                ok: false,
                error: Some(e.to_string()),
            }));
        }
    }

    Ok(Json(TestConnectionResult { ok: true, error: None }))
}
