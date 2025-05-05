use crate::{AppState, db::DbItem};
use age::x25519;
use axum::{body::Bytes, extract::State, http::StatusCode, response::IntoResponse};

pub async fn handle_upload(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Extract X-PubKey header
    let pubkey_b64 = match headers.get("X-PubKey") {
        Some(val) => match val.to_str() {
            Ok(s) => s,
            Err(_) => return (StatusCode::BAD_REQUEST, "Invalid X-PubKey header").into_response(),
        },
        None => return (StatusCode::BAD_REQUEST, "Missing X-PubKey header").into_response(),
    };
    // Validate pubkey (must be valid age X25519 pubkey)
    if pubkey_b64.parse::<x25519::Recipient>().is_err() {
        return (
            StatusCode::BAD_REQUEST,
            "X-PubKey must be a valid age X25519 pubkey",
        )
            .into_response();
    }
    // Validate body (must not be empty)
    if body.is_empty() {
        return (StatusCode::BAD_REQUEST, "Empty body").into_response();
    }
    // Store in DB
    match DbItem::insert(&state.db_pool, pubkey_b64, &body).await {
        Ok(_item) => (StatusCode::CREATED, "ok").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB error: {}", e),
        )
            .into_response(),
    }
}
