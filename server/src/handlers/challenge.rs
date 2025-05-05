use crate::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub pubkey: String,
    pub scope: String,
    pub telegram: Option<String>,
}

pub async fn handle_challenge(
    State(_state): State<AppState>,
    Json(payload): Json<ChallengeRequest>,
) -> impl IntoResponse {
    // Call the actual implementation (to be implemented in challenge_impl)
    // let result = challenge_impl::process_challenge(_state, &payload).await;
    // For now, return a stub response
    (
        StatusCode::OK,
        Json(json!({ "ciphertext": "stub-age-encrypted-jwt" })),
    )
}

// Internal implementation module, not public
mod challenge_impl {
    use super::*;
    use crate::AppState;
    use axum::Json;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[derive(Deserialize)]
    pub struct ChallengeRequest {
        pub pubkey: String,
        pub scope: String,
        pub telegram: Option<String>,
    }

    pub async fn process_challenge(
        _state: &AppState,
        payload: &ChallengeRequest,
    ) -> Result<String, (StatusCode, String)> {
        // 1. Validate pubkey and scope
        // 2. Build JWT claims (sub, aud, iat, exp, telegram?)
        // 3. Sign JWT (HS256)
        // 4. Encrypt JWT with age using pubkey
        // 5. Return ciphertext (base64)
        // TODO: Implement actual logic
        Ok("stub-age-encrypted-jwt".to_string())
    }
}
