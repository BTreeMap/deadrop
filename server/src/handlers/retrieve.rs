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
    let mut items: Vec<DbItem> = Vec::new();
    let mut next_cursor = None;
    // Parse cursor if present
    let cursor_uuid = match &query.cursor {
        Some(cursor_str) => match Uuid::parse_str(cursor_str) {
            Ok(uuid) => Some(uuid),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Invalid cursor"})),
                )
                    .into_response();
            }
        },
        None => None,
    };
    // Query items for pubkey, paginated by created_at DESC, id DESC
    let db_items = if let Some(cursor_id) = cursor_uuid {
        // Get the created_at for the cursor item
        let cursor_item = match DbItem::get_item_by_id(&state.db_pool, cursor_id).await {
            Ok(Some(item)) if item.pubkey == *pubkey => item,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Invalid cursor"})),
                )
                    .into_response();
            }
        };
        // Fetch items created before the cursor's created_at, or same created_at but smaller id
        sqlx::query_as::<_, DbItem>(
            "SELECT * FROM items WHERE pubkey = $1 AND (created_at < $2 OR (created_at = $2 AND id < $3)) ORDER BY created_at DESC, id DESC LIMIT $4"
        )
        .bind(pubkey)
        .bind(cursor_item.created_at)
        .bind(cursor_item.id)
        .bind(page_size as i64)
        .fetch_all(&*state.db_pool)
        .await
        .unwrap_or_default()
    } else {
        // First page
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
        // There may be more items, set next_cursor to the last item's id
        next_cursor = db_items.last().map(|item| item.id.to_string());
    }
    let resp = RetrieveResponse {
        items: item_ids,
        next_cursor,
    };
    (StatusCode::OK, Json(resp)).into_response()
}
