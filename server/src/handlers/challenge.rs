use crate::auth::build_and_encrypt_challenge_jwt;
use crate::{AppState, config::Config};
use age::x25519;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub pubkey: String,
    pub scope: String,
    pub telegram: Option<String>,
}

pub async fn handle_challenge(
    State(state): State<AppState>,
    Json(payload): Json<ChallengeRequest>,
) -> impl IntoResponse {
    match process_challenge(&state.config, &payload).await {
        Ok(ciphertext) => (StatusCode::OK, Json(json!({ "ciphertext": ciphertext }))),
        Err((status, msg)) => (status, Json(json!({ "error": msg }))),
    }
}

// Internal implementation module, not public
async fn process_challenge(
    config: &Config,
    payload: &ChallengeRequest,
) -> Result<String, (StatusCode, String)> {
    // Validate scope
    if payload.scope != "retrieve" && payload.scope != "notify" {
        return Err((StatusCode::BAD_REQUEST, "Invalid scope".to_string()));
    }
    if payload.scope == "notify" && payload.telegram.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Missing telegram for notify scope".to_string(),
        ));
    }
    let aud = format!("/{}", payload.scope);
    let ciphertext = build_and_encrypt_challenge_jwt(
        &payload.pubkey,
        &aud,
        payload.telegram.as_deref(),
        config,
        &payload.pubkey,
        config.jwt_expiration_seconds,
    )?;
    Ok(ciphertext)
}
