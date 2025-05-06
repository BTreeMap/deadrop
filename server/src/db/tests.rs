use crate::db::DbItem;
use dotenvy::dotenv;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;

async fn setup_db() -> PgPool {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");
    // Drop tables for a clean start
    sqlx::query("DROP TABLE IF EXISTS items")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP TABLE IF EXISTS schema_version")
        .execute(&pool)
        .await
        .unwrap();
    // Run migrations
    crate::db::db_migrate(&pool)
        .await
        .expect("Migration failed");
    pool
}

#[tokio::test]
async fn test_insert_and_get_item() {
    let pool = setup_db().await;
    let pubkey = "test_pubkey";
    let ciphertext = b"test_ciphertext";
    let item = DbItem::insert(&pool, pubkey, ciphertext).await.unwrap();
    assert_eq!(item.pubkey, pubkey);
    assert_eq!(item.ciphertext, ciphertext);

    // get by id
    let fetched = DbItem::get_item_by_id(&pool, item.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.id, item.id);
    assert_eq!(fetched.pubkey, pubkey);
    assert_eq!(fetched.ciphertext, ciphertext);
}

#[tokio::test]
async fn test_get_items_for_pubkey() {
    let pool = setup_db().await;
    let pubkey = "test_pubkey2";
    let ciphertext1 = b"cipher1";
    let ciphertext2 = b"cipher2";
    DbItem::insert(&pool, pubkey, ciphertext1).await.unwrap();
    DbItem::insert(&pool, pubkey, ciphertext2).await.unwrap();
    let items = DbItem::get_items_for_pubkey(&pool, pubkey).await.unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.ciphertext == ciphertext1));
    assert!(items.iter().any(|i| i.ciphertext == ciphertext2));
}
