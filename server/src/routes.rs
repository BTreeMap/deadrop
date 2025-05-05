use axum::{
    Router,
    routing::{get, post},
};

// Placeholder handlers - these will be implemented later
async fn upload_handler() -> &'static str {
    "Upload endpoint (not implemented)"
}

async fn challenge_handler() -> &'static str {
    "Challenge endpoint (not implemented)"
}

async fn retrieve_handler() -> &'static str {
    "Retrieve endpoint (not implemented)"
}

async fn download_handler() -> &'static str {
    "Download endpoint (not implemented)"
}

async fn notify_handler() -> &'static str {
    "Notify endpoint (not implemented)"
}

pub fn create_router() -> Router {
    Router::new()
        .route("/upload", post(upload_handler))
        .route("/challenge", post(challenge_handler))
        .route("/retrieve", post(retrieve_handler))
        // Note: download needs a path parameter, handled differently
        .route("/download/:item_id", get(download_handler)) // Placeholder for GET
        .route("/notify", post(notify_handler))
}
