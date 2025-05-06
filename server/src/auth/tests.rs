use super::*;
use age::{Decryptor, Identity, x25519};
use base64::{engine::general_purpose::URL_SAFE, read};
use chrono::Utc;
use serde::de;

fn test_config() -> Config {
    Config {
        host: "127.0.0.1".to_string(),
        port: 12345,
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "test_secret_1234567890".to_string(),
        jwt_expiration_seconds: 60,
        retrieve_page_size: 10,
    }
}

#[test]
fn test_jwt_create_and_verify() {
    let config = test_config();
    let claims = AuthClaims::new(
        "test_pubkey".to_string(),
        "/retrieve".to_string(),
        Utc::now().timestamp() + 60,
        Utc::now().timestamp(),
        None,
    );
    let jwt = create_challenge_jwt(&claims, &config).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let verified = rt
        .block_on(verify_jwt_from_header(&jwt, &config, "/retrieve"))
        .unwrap();
    assert_eq!(verified.sub, claims.sub);
    assert_eq!(verified.aud, claims.aud);
}

#[test]
fn test_age_encrypt_decrypt() {
    // Generate ephemeral keypair
    let id = x25519::Identity::generate();
    let pubkey = id.to_public().to_string();
    let msg = "hello cryptotest!";
    // Encrypt
    let ciphertext_b64 = encrypt_jwt_for_recipient(msg, &pubkey).unwrap();
    let ciphertext = URL_SAFE.decode(ciphertext_b64).unwrap();
    // Decrypt
    let decryptor = Decryptor::new(&ciphertext[..]).unwrap();
    let mut reader = decryptor
        .decrypt(std::iter::once(&id as &dyn Identity))
        .unwrap();
    let mut out = Vec::new();
    std::io::copy(&mut reader, &mut out).unwrap();
    assert_eq!(msg.as_bytes(), &out[..]);
}
