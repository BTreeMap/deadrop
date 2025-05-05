mod config;
pub mod db;
mod handlers;
mod routes;

use crate::config::{AppState, load_config};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Arc::new(load_config().expect("Failed to load configuration"));
    println!("Loaded config: {:?}", config);

    // Create database connection pool
    let db_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url)
            .await
            .expect("Failed to create database pool"),
    );
    println!("Database pool created.");

    // Create application state
    let app_state = AppState {
        db_pool,
        config: Arc::clone(&config),
    };

    // Create router
    let app = routes::create_router(app_state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    println!("Starting server on {}", addr);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
