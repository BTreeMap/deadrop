use chrono::{DateTime, Utc};
use sqlx::{Executor, FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct DbItem {
    pub id: Uuid,
    pub pubkey: String, // base64 encoded X25519 pubkey
    pub ciphertext: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

impl DbItem {
    pub async fn insert(pool: &PgPool, pubkey: &str, ciphertext: &[u8]) -> sqlx::Result<DbItem> {
        let rec = sqlx::query_as::<_, DbItem>(
            "INSERT INTO items (id, pubkey, ciphertext, created_at) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(Uuid::new_v4())
        .bind(pubkey)
        .bind(ciphertext)
        .bind(Utc::now())
        .fetch_one(pool)
        .await?;
        Ok(rec)
    }

    pub async fn get_items_for_pubkey(pool: &PgPool, pubkey: &str) -> sqlx::Result<Vec<DbItem>> {
        sqlx::query_as::<_, DbItem>(
            "SELECT * FROM items WHERE pubkey = $1 ORDER BY created_at DESC",
        )
        .bind(pubkey)
        .fetch_all(pool)
        .await
    }

    pub async fn get_item_by_id(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<DbItem>> {
        sqlx::query_as::<_, DbItem>("SELECT * FROM items WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }
}

/// Run database migrations: create schema_version and items tables if needed.
pub async fn db_migrate(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Create schema_version table if it doesn't exist
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        )
    "#,
    )
    .await?;
    // If schema_version is empty, insert version 0
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM schema_version")
        .fetch_one(pool)
        .await?;
    if count.0 == 0 {
        sqlx::query("INSERT INTO schema_version (version) VALUES (0)")
            .execute(pool)
            .await?;
    }
    // Create items table if it doesn't exist
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id UUID PRIMARY KEY,
            pubkey TEXT NOT NULL,
            ciphertext BYTEA NOT NULL,
            created_at TIMESTAMPTZ NOT NULL
        )
    "#,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
