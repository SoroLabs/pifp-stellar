//! Redis cache helpers for API responses.

use redis::AsyncCommands;
use tracing::{debug, warn};

const CACHE_VERSION_KEY: &str = "indexer:cache:version";

#[derive(Clone)]
pub struct Cache {
    client: redis::Client,
}

impl Cache {
    pub fn new(redis_url: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    async fn get_conn(&self) -> redis::RedisResult<redis::aio::MultiplexedConnection> {
        self.client.get_multiplexed_tokio_connection().await
    }

    pub async fn get_version(&self) -> u64 {
        match self.get_conn().await {
            Ok(mut conn) => conn.get(CACHE_VERSION_KEY).await.unwrap_or(0),
            Err(e) => {
                warn!("Redis unavailable for cache version read: {e}");
                0
            }
        }
    }

    pub async fn invalidate_all(&self) {
        match self.get_conn().await {
            Ok(mut conn) => {
                let incr: redis::RedisResult<i64> = conn.incr(CACHE_VERSION_KEY, 1).await;
                if let Err(e) = incr {
                    warn!("Failed to bump cache version in Redis: {e}");
                } else {
                    debug!("Cache version bumped after indexing update");
                }
            }
            Err(e) => warn!("Redis unavailable for cache invalidation: {e}"),
        }
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("Redis unavailable during get for key `{key}`: {e}");
                return None;
            }
        };

        let cached: Option<String> = match conn.get(key).await {
            Ok(v) => v,
            Err(e) => {
                warn!("Redis GET failed for key `{key}`: {e}");
                return None;
            }
        };

        cached.and_then(|v| serde_json::from_str::<T>(&v).ok())
    }

    pub async fn set_json<T: serde::Serialize>(&self, key: &str, value: &T, ttl_secs: u64) {
        let payload = match serde_json::to_string(value) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to serialize cache payload for key `{key}`: {e}");
                return;
            }
        };

        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!("Redis unavailable during set for key `{key}`: {e}");
                return;
            }
        };

        let ttl: u64 = ttl_secs.max(1);
        if let Err(e) = conn.set_ex::<_, _, ()>(key, payload, ttl).await {
            warn!("Redis SETEX failed for key `{key}`: {e}");
        }
    }
}
