use crate::auth::{AuthClaims, verify_jwt_from_header};
use crate::db::DbItem;
use crate::{AppState, db};
use axum::body::Body;
use axum::http::Request;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use uuid::Uuid;

pub async fn handle_download(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    // Verify JWT and extract claims
    let jwt = auth_header.0.token();
    let claims = match verify_jwt_from_header(jwt, &state.config, "/retrieve").await {
        Ok(c) => c,
        Err((status, msg)) => return (status, msg).into_response(),
    };
    // Check if item_id belongs to the user (claims.sub)
    let owner_pubkey = &claims.sub;
    let uuid = match Uuid::parse_str(&item_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid UUID format").into_response(),
    };
    let item = match DbItem::get_item_by_id(&state.db_pool, uuid).await {
        Ok(Some(item)) => {
            if item.pubkey != *owner_pubkey {
                return (StatusCode::NOT_FOUND, "Item not found").into_response();
            }
            item
        }
        Ok(None) => return (StatusCode::NOT_FOUND, "Item not found").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "DB error").into_response(),
    };
    // Return ciphertext as binary
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        item.ciphertext,
    )
        .into_response()
}
