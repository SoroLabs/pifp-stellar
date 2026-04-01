//! Webhook dispatching with HMAC signing and retry/backoff.

use std::time::Instant;

use async_trait::async_trait;
use hmac::{Hmac, KeyInit, Mac};
use reqwest::Client;
use sha2::Sha256;
use tracing::{error, info, warn};

use crate::db;
use crate::events::PifpEvent;

const MAX_RETRIES: u32 = 4;
const BASE_BACKOFF_MS: u64 = 500;

#[derive(Clone)]
pub struct DispatchContext {
    pub pool: sqlx::SqlitePool,
    pub client: Client,
}

#[async_trait]
trait WebhookSender {
    async fn post(
        &self,
        url: &str,
        signature: &str,
        event_type: &str,
        body: &str,
    ) -> Result<u16, String>;
}

struct ReqwestSender {
    client: Client,
}

#[async_trait]
impl WebhookSender for ReqwestSender {
    async fn post(
        &self,
        url: &str,
        signature: &str,
        event_type: &str,
        body: &str,
    ) -> Result<u16, String> {
        let response = self
            .client
            .post(url)
            .header("content-type", "application/json")
            .header("x-pifp-signature", signature)
            .header("x-pifp-event-type", event_type)
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(response.status().as_u16())
    }
}

pub async fn dispatch_event(ctx: DispatchContext, event: PifpEvent) {
    let targets = match db::get_webhooks_for_event(&ctx.pool, &event.event_type).await {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to load webhook targets: {e}");
            return;
        }
    };

    if targets.is_empty() {
        return;
    }

    let payload = match serde_json::to_string(&event) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to serialize webhook payload: {e}");
            return;
        }
    };

    let sender = ReqwestSender {
        client: ctx.client.clone(),
    };
    for target in targets {
        let signature = sign_payload(&target.secret, &payload);
        deliver_with_retry(&ctx.pool, &sender, &target, &payload, &signature).await;
    }
}

async fn deliver_with_retry<S: WebhookSender + Sync>(
    pool: &sqlx::SqlitePool,
    sender: &S,
    target: &db::WebhookTarget,
    payload: &str,
    signature: &str,
) {
    for attempt in 1..=MAX_RETRIES {
        let started = Instant::now();
        let result = sender
            .post(&target.url, signature, &target.event_type, payload)
            .await;
        let elapsed_ms = started.elapsed().as_millis() as i64;
        let (status_code, success, error) = match result {
            Ok(status) if (200..300).contains(&status) => (Some(status as i32), true, None),
            Ok(status) => (
                Some(status as i32),
                false,
                Some(format!("Non-success HTTP status: {status}")),
            ),
            Err(err) => (None, false, Some(err)),
        };

        if let Err(e) = db::log_webhook_delivery_attempt(
            pool,
            target.webhook_id,
            &target.event_type,
            payload,
            status_code,
            success,
            attempt as i32,
            error.as_deref(),
            elapsed_ms,
        )
        .await
        {
            warn!("Failed to persist webhook delivery attempt: {e}");
        }

        if success {
            info!(
                "Webhook delivered: webhook_id={} event_type={} attempt={}",
                target.webhook_id, target.event_type, attempt
            );
            return;
        }

        if attempt < MAX_RETRIES {
            let backoff_ms = BASE_BACKOFF_MS * (1u64 << (attempt - 1));
            tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
        } else {
            warn!(
                "Webhook delivery failed after retries: webhook_id={} event_type={}",
                target.webhook_id, target.event_type
            );
        }
    }
}

pub fn sign_payload(secret: &str, payload: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts keys of any size for SHA-256");
    mac.update(payload.as_bytes());
    let sig = mac.finalize().into_bytes();
    format!("sha256={}", hex::encode(sig))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::Mutex;

    #[test]
    fn test_sign_payload_format() {
        let sig = sign_payload("secret", r#"{"event":"ok"}"#);
        assert!(sig.starts_with("sha256="));
        assert_eq!(sig.len(), "sha256=".len() + 64);
    }

    struct MockSender {
        responses: Mutex<Vec<Result<u16, String>>>,
    }

    #[async_trait]
    impl WebhookSender for MockSender {
        async fn post(
            &self,
            _url: &str,
            _signature: &str,
            _event_type: &str,
            _body: &str,
        ) -> Result<u16, String> {
            self.responses.lock().unwrap().remove(0)
        }
    }

    async fn setup_delivery_db() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE webhooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                secret TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE webhook_subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                webhook_id INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                UNIQUE(webhook_id, event_type)
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE webhook_deliveries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                webhook_id INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                status_code INTEGER,
                success INTEGER NOT NULL DEFAULT 0,
                attempt INTEGER NOT NULL,
                error TEXT,
                latency_ms INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_retry_backoff_until_success() {
        let pool = setup_delivery_db().await;
        let sender = MockSender {
            responses: Mutex::new(vec![Ok(500), Err("network".to_string()), Ok(200)]),
        };
        let target = db::WebhookTarget {
            webhook_id: 1,
            url: "https://example.com".to_string(),
            secret: "s".to_string(),
            event_type: "project_active".to_string(),
        };
        deliver_with_retry(
            &pool,
            &sender,
            &target,
            r#"{"event_type":"project_active"}"#,
            "sha256=abc",
        )
        .await;
        let deliveries = db::count_webhook_deliveries(&pool, 1).await.unwrap();
        assert_eq!(deliveries, 3);
    }

    #[tokio::test]
    #[ignore = "network dependent"]
    async fn test_dispatch_to_httpbin_request_bin() {
        let pool = setup_delivery_db().await;
        db::create_webhook(
            &pool,
            &db::NewWebhookRegistration {
                url: "https://httpbin.org/post".to_string(),
                secret: "integration-secret".to_string(),
                event_types: vec!["project_active".to_string()],
            },
        )
        .await
        .unwrap();

        let ctx = DispatchContext {
            pool: pool.clone(),
            client: Client::new(),
        };
        dispatch_event(
            ctx,
            PifpEvent {
                event_type: "project_active".to_string(),
                project_id: Some("42".to_string()),
                actor: Some("GABC".to_string()),
                amount: Some("1000".to_string()),
                extra_data: None,
                ledger: 1,
                timestamp: 1,
                contract_id: "CID".to_string(),
                tx_hash: Some("TX".to_string()),
            },
        )
        .await;

        let success_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM webhook_deliveries WHERE success = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(success_count.0 >= 1);
    }
}
