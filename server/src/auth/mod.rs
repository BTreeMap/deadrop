use crate::config::Config;
use age::{Encryptor, Recipient, x25519};
use axum::http::StatusCode;
use axum_extra::headers::Authorization;
use base64::Engine;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthClaims {
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub telegram: Option<String>,
}

impl AuthClaims {
    pub fn new(sub: String, aud: String, exp: i64, iat: i64, telegram: Option<String>) -> Self {
        AuthClaims {
            sub,
            aud,
            exp,
            iat,
            telegram,
        }
    }
}

pub async fn verify_jwt_from_header(
    jwt: &str,
    config: &Config,
    expected_aud: &str,
) -> Result<AuthClaims, (StatusCode, String)> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[expected_aud]);
    let token_data = decode::<AuthClaims>(
        jwt,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| (StatusCode::UNAUTHORIZED, format!("JWT error: {}", e)))?;
    // Check expiration
    let now = chrono::Utc::now().timestamp();
    if token_data.claims.exp < now {
        return Err((StatusCode::UNAUTHORIZED, "JWT expired".to_string()));
    }
    Ok(token_data.claims)
}

/// Create and sign a JWT for the challenge
pub fn create_challenge_jwt(
    claims: &AuthClaims,
    config: &Config,
) -> Result<String, (StatusCode, String)> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("JWT error: {}", e),
        )
    })
}

/// Encrypt a JWT with age for the given recipient public key (base64)
pub fn encrypt_jwt_for_recipient(
    jwt: &str,
    recipient_pubkey_b64: &str,
) -> Result<String, (StatusCode, String)> {
    let recipient = recipient_pubkey_b64
        .parse::<x25519::Recipient>()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid pubkey: {}", e)))?;
    let recipients: Vec<Box<dyn Recipient + Send>> = vec![Box::new(recipient)];
    let encryptor =
        Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref() as &dyn Recipient))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Age encryption error: {}", e),
                )
            })?;
    let mut encrypted_jwt = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted_jwt).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Age encryption error: {}", e),
        )
    })?;
    writer.write_all(jwt.as_bytes()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Age write error: {}", e),
        )
    })?;
    writer.finish().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Age finish error: {}", e),
        )
    })?;
    // Base64 encode the ciphertext
    Ok(base64::engine::general_purpose::URL_SAFE.encode(&encrypted_jwt))
}

/// Full challenge cryptography: build claims, sign JWT, encrypt for recipient
pub fn build_and_encrypt_challenge_jwt(
    sub: &str,
    aud: &str,
    telegram: Option<&str>,
    config: &Config,
    recipient_pubkey_b64: &str,
    ttl_secs: i64,
) -> Result<String, (StatusCode, String)> {
    let now = Utc::now().timestamp();
    let exp = now + ttl_secs;
    let claims = AuthClaims {
        sub: sub.to_string(),
        aud: aud.to_string(),
        exp,
        iat: now,
        telegram: telegram.map(|s| s.to_string()),
    };
    let jwt = create_challenge_jwt(&claims, config)?;
    encrypt_jwt_for_recipient(&jwt, recipient_pubkey_b64)
}
