use serde::Deserialize;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_seconds: i64,
    #[serde(default = "default_retrieve_page_size")]
    pub retrieve_page_size: u32, // New: default page size for /retrieve
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    63460
}

fn default_jwt_expiration() -> i64 {
    300 // 5 minutes default
}

fn default_retrieve_page_size() -> u32 {
    50 // Default page size for pagination
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<Pool<Postgres>>,
    pub config: Arc<Config>,
}

pub fn load_config() -> Result<Config, envy::Error> {
    dotenvy::dotenv().ok(); // Load .env file if present
    envy::from_env::<Config>()
}
