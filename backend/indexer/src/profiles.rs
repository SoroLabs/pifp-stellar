//! Profile storage — CRUD for off-chain user identity metadata.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::errors::Result;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Profile {
    pub address: String,
    pub nickname: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ProfileUpdate {
    pub nickname: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

pub async fn upsert_profile(
    pool: &SqlitePool,
    address: &str,
    update: &ProfileUpdate,
) -> Result<Profile> {
    sqlx::query(
        r#"
        INSERT INTO profiles (address, nickname, bio, avatar_url, updated_at)
        VALUES (?1, ?2, ?3, ?4, strftime('%s', 'now'))
        ON CONFLICT(address) DO UPDATE SET
            nickname   = excluded.nickname,
            bio        = excluded.bio,
            avatar_url = excluded.avatar_url,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(address)
    .bind(&update.nickname)
    .bind(&update.bio)
    .bind(&update.avatar_url)
    .execute(pool)
    .await?;

    get_profile(pool, address).await.map(|p| p.unwrap())
}

pub async fn get_profile(pool: &SqlitePool, address: &str) -> Result<Option<Profile>> {
    let row = sqlx::query_as::<_, Profile>(
        "SELECT address, nickname, bio, avatar_url, updated_at FROM profiles WHERE address = ?1",
    )
    .bind(address)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn delete_profile(pool: &SqlitePool, address: &str) -> Result<bool> {
    let res = sqlx::query("DELETE FROM profiles WHERE address = ?1")
        .bind(address)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
