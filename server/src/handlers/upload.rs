use axum::{http::StatusCode, response::IntoResponse};

pub async fn handle_upload() -> impl IntoResponse {
    // Call the actual implementation here (to be implemented in another module)
    (StatusCode::CREATED, "upload handler stub")
}
