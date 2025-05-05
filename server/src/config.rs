use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_host")]
    pub server_host: String,
    #[serde(default = "default_port")]
    pub server_port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration_seconds: i64,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_jwt_expiration() -> i64 {
    300 // 5 minutes
}

impl Config {
    pub fn load() -> Result<Self, envy::Error> {
        dotenvy::dotenv().ok(); // Load .env file if present
        envy::from_env::<Config>()
    }

    pub fn server_addr(&self) -> Result<SocketAddr, std::net::AddrParseError> {
        format!("{}:{}", self.server_host, self.server_port).parse()
    }
}
