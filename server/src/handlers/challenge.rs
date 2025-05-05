use crate::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
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
    match process_challenge(&state, &payload).await {
        Ok(ciphertext) => (StatusCode::OK, Json(json!({ "ciphertext": ciphertext }))),
        Err((status, msg)) => (status, Json(json!({ "error": msg }))),
    }
}

// Internal implementation module, not public
async fn process_challenge(
    state: &AppState,
    payload: &ChallengeRequest,
) -> Result<String, (StatusCode, String)> {
    // Validate scope
    if payload.scope != "retrieve" && payload.scope != "notify" {
        return Err((StatusCode::BAD_REQUEST, "Invalid scope".to_string()));
    }
    // Validate pubkey (basic check, should be base64 X25519)
    if payload.pubkey.len() < 32 {
        return Err((StatusCode::BAD_REQUEST, "Invalid pubkey".to_string()));
    }
    // JWT claims
    let now = Utc::now().timestamp();
    let exp = now + 300; // 5 minutes
    let mut claims = serde_json::Map::new();
    claims.insert("sub".to_string(), json!(payload.pubkey));
    claims.insert("aud".to_string(), json!(format!("/{}", payload.scope)));
    claims.insert("iat".to_string(), json!(now));
    claims.insert("exp".to_string(), json!(exp));
    if payload.scope == "notify" {
        if let Some(tg) = &payload.telegram {
            claims.insert("telegram".to_string(), json!(tg));
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                "Missing telegram for notify scope".to_string(),
            ));
        }
    }
    // Sign JWT (HS256)
    let jwt = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("JWT error: {}", e),
        )
    })?;
    // Encrypt JWT with age (X25519) - placeholder
    // TODO: Use an age crate to encrypt the JWT for payload.pubkey
    let ciphertext = format!("age({})", jwt); // Placeholder
    Ok(ciphertext)
}
