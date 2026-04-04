use ai_providers::ProviderRegistry;
use sqlx::PgPool;
use std::sync::Arc;

use crate::job_events::JobEvent;

/// Shared application state injected into every route handler via Axum's
/// `State` extractor. All fields must be `Clone + Send + Sync`.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// AI provider registry — resolves `provider_kind` slugs to adapters.
    pub providers: ProviderRegistry,
    /// In-process cache for hot reads (optional; ephemeral — safe to lose on restart).
    #[allow(dead_code)]
    pub cache: Arc<moka::future::Cache<String, serde_json::Value>>,
    /// Broadcast channel for live agent-job events (streaming tokens, status).
    /// The worker publishes; SSE handlers subscribe.
    pub events_tx: tokio::sync::broadcast::Sender<JobEvent>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let cache = moka::future::Cache::builder()
            .max_capacity(1_000)
            .time_to_live(std::time::Duration::from_secs(30))
            .build();

        // 2048 slots — plenty for bursts of token events across concurrent jobs.
        let (events_tx, _) = tokio::sync::broadcast::channel(2048);

        Self {
            pool,
            providers: ProviderRegistry::new(),
            cache: Arc::new(cache),
            events_tx,
        }
    }
}
