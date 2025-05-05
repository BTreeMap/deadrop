use crate::config::AppState;
use crate::handlers;
use axum::{
    Router,
    routing::{get, post},
};

pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .route("/upload", post(handlers::upload::handle_upload))
        .route("/challenge", post(handlers::challenge::handle_challenge))
        .route("/retrieve", post(handlers::retrieve::handle_retrieve))
        .route(
            "/download/:item_id",
            get(handlers::download::handle_download),
        )
        .route("/notify", post(handlers::notify::handle_notify))
        .with_state(app_state)
}
