/// App-managed PostgreSQL lifecycle.
///
/// On first run:
///   1. Chooses a high port and records it in `data_dir/db.port`.
///   2. Downloads the postgres binary and runs `initdb` (pg_embed handles this).
///   3. Starts the postgres process.
///
/// On subsequent runs:
///   1. Reads the persisted port.
///   2. Starts postgres against the same data directory.
///
/// Developers may skip this entirely by setting `DATABASE_URL` in the
/// environment — this struct is never constructed in that code path.
use anyhow::{Context, Result};
use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_fetch::{PgFetchSettings, PG_V15};
use pg_embed::postgres::{PgEmbed, PgSettings};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, warn};

const DB_NAME: &str = "yaf";
const DB_USER: &str = "yaf";
const DB_PASSWORD: &str = "yaf_local";
const PORT_FILE: &str = "db.port";
const DEFAULT_PORT: u16 = 54320;

pub struct EmbeddedPostgres {
    // Kept alive for the process lifetime — dropping it stops the DB.
    #[allow(dead_code)]
    pub pg: PgEmbed,
    pub database_url: String,
}

impl EmbeddedPostgres {
    /// Start (or restart) the embedded PostgreSQL server.
    pub async fn start(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)
            .with_context(|| format!("create data dir {}", data_dir.display()))?;

        let port = resolve_port(data_dir)?;
        let pg_data_dir = data_dir.join("pgdata");

        info!(
            port,
            data_dir = %data_dir.display(),
            "starting embedded PostgreSQL"
        );

        let pg_settings = PgSettings {
            database_dir: pg_data_dir,
            port,
            user: DB_USER.to_string(),
            password: DB_PASSWORD.to_string(),
            auth_method: PgAuthMethod::Plain,
            persistent: true,
            timeout: Some(Duration::from_secs(60)),
            migration_dir: None,
        };

        let fetch_settings = PgFetchSettings {
            version: PG_V15,
            ..Default::default()
        };

        let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;

        // setup() downloads the binary on first run and runs initdb if needed.
        pg.setup().await.context("pg_embed setup")?;
        pg.start_db().await.context("pg_embed start")?;

        // create_database is not truly idempotent; ignore "already exists".
        match pg.create_database(DB_NAME).await {
            Ok(_) => {}
            Err(e) if e.to_string().contains("already exists") => {}
            Err(e) => return Err(e).context("create yaf database"),
        }

        let database_url = pg.full_db_uri(DB_NAME);
        info!(%database_url, "embedded PostgreSQL ready");

        Ok(Self { pg, database_url })
    }

    pub async fn stop(&mut self) -> Result<()> {
        warn!("stopping embedded PostgreSQL");
        self.pg.stop_db().await.context("stop embedded postgres")?;
        Ok(())
    }
}

fn resolve_port(data_dir: &Path) -> Result<u16> {
    let port_file: PathBuf = data_dir.join(PORT_FILE);
    if port_file.exists() {
        let raw = std::fs::read_to_string(&port_file)
            .with_context(|| format!("read {}", port_file.display()))?;
        let port: u16 = raw
            .trim()
            .parse()
            .with_context(|| format!("parse port from {}", port_file.display()))?;
        return Ok(port);
    }

    // Pick a free port and persist it so subsequent runs reuse the same data directory.
    let port = find_free_port().unwrap_or(DEFAULT_PORT);
    std::fs::write(&port_file, port.to_string())
        .with_context(|| format!("write {}", port_file.display()))?;
    Ok(port)
}

/// Find a free TCP port on loopback (best-effort).
fn find_free_port() -> Option<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    Some(listener.local_addr().ok()?.port())
}
