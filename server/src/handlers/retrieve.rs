use crate::{AppState, auth::verify_jwt_from_header, db::DbItem};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::{DateTime, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct RetrieveQuery {
    cursor: Option<String>,
}

#[derive(Serialize)]
pub struct RetrieveResponse {
    items: Vec<String>, // item IDs as strings
    next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CursorClaims {
    exp: usize,
    scope: String, // should be "/retrieve-cursor"
    created_at: DateTime<Utc>,
    id: uuid::Uuid,
}

pub async fn handle_retrieve(
    State(state): State<AppState>,
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    Query(query): Query<RetrieveQuery>,
) -> impl IntoResponse {
    // Verify JWT and extract claims
    let jwt = auth_header.0.token();
    let claims = match verify_jwt_from_header(jwt, &state.config, "/retrieve").await {
        Ok(c) => c,
        Err((status, msg)) => {
            return (status, Json(serde_json::json!({"error": msg}))).into_response();
        }
    };
    let pubkey = &claims.sub;
    let page_size = state.config.retrieve_page_size as usize;
    let mut next_cursor = None;
    let (created_at_cursor, id_cursor) = if let Some(cursor_str) = &query.cursor {
        // Decode and verify cursor JWT
        match decode::<CursorClaims>(
            cursor_str,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(token_data) if token_data.claims.scope == "/retrieve-cursor" => (
                Some(token_data.claims.created_at),
                Some(token_data.claims.id),
            ),
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Invalid cursor"})),
                )
                    .into_response();
            }
        }
    } else {
        (None, None)
    };
    // Query items for pubkey, paginated by created_at DESC, id DESC
    let db_items = if let (Some(created_at), Some(id)) = (created_at_cursor, id_cursor) {
        sqlx::query_as::<_, DbItem>(
            "SELECT * FROM items WHERE pubkey = $1 AND (created_at < $2 OR (created_at = $2 AND id < $3)) ORDER BY created_at DESC, id DESC LIMIT $4"
        )
        .bind(pubkey)
        .bind(created_at)
        .bind(id)
        .bind(page_size as i64)
        .fetch_all(&*state.db_pool)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query_as::<_, DbItem>(
            "SELECT * FROM items WHERE pubkey = $1 ORDER BY created_at DESC, id DESC LIMIT $2",
        )
        .bind(pubkey)
        .bind(page_size as i64)
        .fetch_all(&*state.db_pool)
        .await
        .unwrap_or_default()
    };
    let item_ids: Vec<String> = db_items.iter().map(|item| item.id.to_string()).collect();
    if db_items.len() == page_size {
        if let Some(last) = db_items.last() {
            // Create a signed cursor JWT
            let claims = CursorClaims {
                exp: (Utc::now() + chrono::Duration::minutes(10)).timestamp() as usize,
                scope: "/retrieve-cursor".to_string(),
                created_at: last.created_at,
                id: last.id,
            };
            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            )
            .unwrap();
            next_cursor = Some(token);
        }
    }
    let resp = RetrieveResponse {
        items: item_ids,
        next_cursor,
    };
    (StatusCode::OK, Json(resp)).into_response()
}
