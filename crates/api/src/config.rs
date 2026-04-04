use anyhow::Result;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP bind address (default: 127.0.0.1)
    #[serde(default = "default_host")]
    pub api_host: String,

    /// HTTP port (default: 3001)
    #[serde(default = "default_port")]
    pub api_port: u16,

    /// If set, use this external PostgreSQL URL and skip embedded DB.
    /// Useful for developers who run `docker compose up -d`.
    pub database_url: Option<String>,

    /// Directory for embedded Postgres data + app config.
    /// Defaults to OS-appropriate app data directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3001
}

impl Config {
    pub fn load() -> Result<Self> {
        let cfg: Config = Figment::new()
            // Lowest priority: defaults encoded in the struct
            .merge(Toml::file("youarefounder.toml").nested())
            // Highest priority: environment variables
            .merge(Env::raw())
            .extract()?;
        Ok(cfg)
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.api_host, self.api_port)
    }

    /// Resolve the data directory, falling back to OS app-data path.
    pub fn resolved_data_dir(&self) -> PathBuf {
        if let Some(dir) = &self.data_dir {
            return dir.clone();
        }

        directories::ProjectDirs::from("com", "youarefounder", "youarefounder")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("data")
            })
    }
}
