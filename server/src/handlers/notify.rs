use axum::{http::StatusCode, response::IntoResponse};

pub async fn handle_notify() -> impl IntoResponse {
    // Call the actual implementation here (to be implemented in another module)
    (StatusCode::OK, "notify handler stub")
}
