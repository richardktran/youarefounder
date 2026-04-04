pub mod ai_profile;
pub mod company;
pub mod hiring;
pub mod job;
pub mod person;
pub mod product;
pub mod ticket;
pub mod workspace;
pub mod workspace_member;

use anyhow::Result;
use sqlx::PgPool;

pub use sqlx::postgres::PgPoolOptions;

/// Run all pending SQLx migrations embedded in this crate.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    tracing::info!("running database migrations");
    sqlx::migrate!("./migrations").run(pool).await?;
    tracing::info!("migrations complete");
    Ok(())
}

/// Build a connection pool.
pub async fn connect(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}
