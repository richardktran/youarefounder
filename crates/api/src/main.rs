mod config;
mod embedded_postgres;
mod error;
mod job_events;
mod routes;
mod state;
mod worker;

use anyhow::{Context, Result};
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    // ── Observability ──────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".parse().unwrap()),
        )
        .init();

    // ── Config ─────────────────────────────────────────────────────────────────
    let config = Config::load().context("load config")?;
    info!(
        host = %config.api_host,
        port = config.api_port,
        "starting youarefounder API"
    );

    // ── Database ───────────────────────────────────────────────────────────────
    // Two paths:
    //   1. DATABASE_URL is set → use it (Docker Compose / developer mode).
    //   2. No DATABASE_URL → start embedded PostgreSQL.
    let (database_url, mut embedded) = resolve_database(&config).await?;

    let pool = db::connect(&database_url)
        .await
        .context("connect to database")?;

    db::run_migrations(&pool)
        .await
        .context("run migrations")?;

    // ── App state ─────────────────────────────────────────────────────────────
    let state = AppState::new(pool);

    // ── Worker ─────────────────────────────────────────────────────────────────
    worker::spawn(
        state.pool.clone(),
        state.providers.clone(),
        state.events_tx.clone(),
    );

    // ── Router ─────────────────────────────────────────────────────────────────
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/v1", routes::v1_router())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    // ── Serve ──────────────────────────────────────────────────────────────────
    let addr: SocketAddr = config
        .bind_addr()
        .parse()
        .context("parse bind address")?;

    info!(%addr, "API listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("bind TCP listener")?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve")?;

    if let Some(ref mut pg) = embedded {
        pg.stop().await.ok();
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

/// Returns `(database_url, embedded_handle)`.
/// The embedded handle is kept alive for the duration of the process.
async fn resolve_database(
    config: &Config,
) -> Result<(String, Option<embedded_postgres::EmbeddedPostgres>)> {
    if let Some(url) = &config.database_url {
        info!(mode = "external", "using external PostgreSQL");
        return Ok((url.clone(), None));
    }

    info!(mode = "embedded", "starting embedded PostgreSQL");
    let data_dir = config.resolved_data_dir();
    let embedded = embedded_postgres::EmbeddedPostgres::start(&data_dir)
        .await
        .context("start embedded postgres")?;

    Ok((embedded.database_url.clone(), Some(embedded)))
}
