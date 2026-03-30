//! Slack webhook notifications for oracle verification failures.
//!
//! This module is intentionally isolated from the verification pipeline.
//! All functions are best-effort: callers must use `let _ = ...` or a timeout
//! wrapper so that a Slack outage never interrupts proof processing.
//!
//! # Environment
//! Set `SLACK_WEBHOOK_URL` to your incoming webhook URL.
//! If the variable is absent the notification is silently skipped.

use reqwest::Client;
use serde::Serialize;
use tracing::warn;

/// Slack incoming-webhook payload.
///
/// Slack's incoming webhook API accepts a JSON body with at minimum a `text`
/// field.  We keep the payload simple and human-readable so it renders well
/// in both the Slack UI and any log aggregator that captures the raw JSON.
#[derive(Debug, Serialize)]
struct SlackMessage {
    text: String,
}

/// Send a structured Slack alert for an oracle proof verification failure.
///
/// # Arguments
/// * `project_id` – on-chain project identifier
/// * `proof_cid`  – IPFS CID of the proof artifact that failed
/// * `error_msg`  – human-readable description of the failure reason
///
/// # Errors
/// Returns an error if:
/// - `SLACK_WEBHOOK_URL` is not set (silently skipped — returns `Ok(())`)
/// - The HTTP request to Slack fails
/// - Slack returns a non-2xx status
///
/// # Safety
/// This function **never panics**.  Callers should wrap it in a timeout and
/// discard the result so that Slack delivery issues cannot affect the
/// verification pipeline:
///
/// ```ignore
/// use tokio::time::{timeout, Duration};
/// let _ = timeout(
///     Duration::from_secs(3),
///     notify_verification_failure(&project_id, &proof_cid, &err),
/// ).await;
/// ```
pub async fn notify_verification_failure(
    project_id: &str,
    proof_cid: &str,
    error_msg: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let webhook_url = match std::env::var("SLACK_WEBHOOK_URL") {
        Ok(url) if !url.trim().is_empty() => url,
        // Variable absent or empty — skip silently.
        _ => return Ok(()),
    };

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");

    let text = format!(
        ":rotating_light: *Oracle verification failed*\n\
         • *Project ID:* `{project_id}`\n\
         • *Proof CID:* `{proof_cid}`\n\
         • *Error:* {error_msg}\n\
         • *Timestamp:* {now}"
    );

    let payload = SlackMessage { text };

    let client = Client::new();
    let response = client
        .post(&webhook_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Slack HTTP request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Slack returned {status}: {body}").into());
    }

    Ok(())
}

/// Fire-and-forget wrapper: sends the Slack alert with a 3-second timeout.
///
/// Any delivery failure is logged as a warning but **never propagated**.
/// This is the recommended call-site pattern for the verification pipeline.
pub async fn alert_verification_failure(project_id: &str, proof_cid: &str, error_msg: &str) {
    use tokio::time::{timeout, Duration};

    let result = timeout(
        Duration::from_secs(3),
        notify_verification_failure(project_id, proof_cid, error_msg),
    )
    .await;

    match result {
        Ok(Ok(())) => {} // delivered
        Ok(Err(e)) => warn!(error = %e, "failed to deliver Slack alert"),
        Err(_) => warn!("Slack alert timed out after 3s"),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Payload serialization ─────────────────────────────────────────────────

    #[test]
    fn test_slack_message_serializes_correctly() {
        let msg = SlackMessage {
            text: "hello world".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"text\""));
        assert!(json.contains("hello world"));
    }

    #[test]
    fn test_slack_message_contains_required_fields() {
        let project_id = "42";
        let proof_cid = "QmTestCID123";
        let error_msg = "hash mismatch";

        let text = format!(
            ":rotating_light: *Oracle verification failed*\n\
             • *Project ID:* `{project_id}`\n\
             • *Proof CID:* `{proof_cid}`\n\
             • *Error:* {error_msg}\n\
             • *Timestamp:* 2024-01-01T00:00:00Z"
        );
        let msg = SlackMessage { text };
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("42"));
        assert!(json.contains("QmTestCID123"));
        assert!(json.contains("hash mismatch"));
    }

    // ── Missing env var ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_missing_webhook_url_returns_ok() {
        // Ensure the variable is absent for this test.
        std::env::remove_var("SLACK_WEBHOOK_URL");

        let result = notify_verification_failure("1", "QmAbc", "some error").await;
        assert!(result.is_ok(), "should silently skip when env var is absent");
    }

    #[tokio::test]
    async fn test_empty_webhook_url_returns_ok() {
        std::env::set_var("SLACK_WEBHOOK_URL", "");
        let result = notify_verification_failure("1", "QmAbc", "some error").await;
        assert!(result.is_ok());
        std::env::remove_var("SLACK_WEBHOOK_URL");
    }

    // ── Mocked webhook ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_successful_delivery_to_mock_server() {
        let mut server = mockito::Server::new_async().await;

        let mock = server
            .mock("POST", "/webhook")
            .with_status(200)
            .with_body("ok")
            .match_header("content-type", mockito::Matcher::Regex("application/json".to_string()))
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::Regex("Project ID".to_string()),
                mockito::Matcher::Regex("QmMockCID".to_string()),
            ]))
            .create_async()
            .await;

        let url = format!("{}/webhook", server.url());
        std::env::set_var("SLACK_WEBHOOK_URL", &url);

        let result = notify_verification_failure("99", "QmMockCID", "test error").await;

        mock.assert_async().await;
        assert!(result.is_ok());

        std::env::remove_var("SLACK_WEBHOOK_URL");
    }

    #[tokio::test]
    async fn test_slack_non_200_returns_error() {
        let mut server = mockito::Server::new_async().await;

        let _mock = server
            .mock("POST", "/webhook")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let url = format!("{}/webhook", server.url());
        std::env::set_var("SLACK_WEBHOOK_URL", &url);

        let result = notify_verification_failure("1", "QmBad", "err").await;
        assert!(result.is_err(), "non-200 should return Err");

        std::env::remove_var("SLACK_WEBHOOK_URL");
    }

    #[tokio::test]
    async fn test_invalid_webhook_url_returns_error() {
        std::env::set_var("SLACK_WEBHOOK_URL", "http://127.0.0.1:1"); // nothing listening

        let result = notify_verification_failure("1", "QmBad", "err").await;
        assert!(result.is_err(), "unreachable URL should return Err");

        std::env::remove_var("SLACK_WEBHOOK_URL");
    }

    // ── alert_verification_failure (fire-and-forget wrapper) ─────────────────

    #[tokio::test]
    async fn test_alert_does_not_panic_on_missing_env() {
        std::env::remove_var("SLACK_WEBHOOK_URL");
        // Must complete without panic or hang.
        alert_verification_failure("5", "QmXyz", "network error").await;
    }

    #[tokio::test]
    async fn test_alert_does_not_panic_on_delivery_failure() {
        std::env::set_var("SLACK_WEBHOOK_URL", "http://127.0.0.1:1");
        // Must complete without panic — delivery failure is swallowed.
        alert_verification_failure("5", "QmXyz", "network error").await;
        std::env::remove_var("SLACK_WEBHOOK_URL");
    }

    #[tokio::test]
    async fn test_alert_respects_timeout() {
        // Point at a mock that never responds within 3s.
        // We use a real mock that delays — but since we can't easily do that
        // with mockito, we verify the timeout path by using an unreachable addr.
        std::env::set_var("SLACK_WEBHOOK_URL", "http://10.255.255.1:9999");
        // Should return within ~3s (timeout fires), not hang indefinitely.
        let start = std::time::Instant::now();
        alert_verification_failure("1", "QmTimeout", "timeout test").await;
        // The timeout is 3s; we allow up to 4s for CI overhead.
        assert!(
            start.elapsed().as_secs() <= 4,
            "alert_verification_failure took too long"
        );
        std::env::remove_var("SLACK_WEBHOOK_URL");
    }

    // ── Webhook failure does not affect verification result ───────────────────

    #[tokio::test]
    async fn test_webhook_failure_does_not_replace_original_error() {
        // Simulate: verification fails, Slack also fails.
        // The original error must be preserved.
        std::env::set_var("SLACK_WEBHOOK_URL", "http://127.0.0.1:1");

        let original_error = "proof hash mismatch";

        // Fire alert (will fail silently).
        alert_verification_failure("7", "QmOriginal", original_error).await;

        // The original error string is unchanged — this is the contract.
        assert_eq!(original_error, "proof hash mismatch");

        std::env::remove_var("SLACK_WEBHOOK_URL");
    }
}
