//! Soroban RPC client — polls `getEvents` and decodes PIFP events.
//!
//! ## Resilience
//!
//! * Exponential back-off is applied when the RPC returns an error or rate-limit
//!   response, up to [`MAX_BACKOFF_SECS`] seconds.
//! * Transient network errors (connection reset, timeout) are retried silently.

use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::errors::{IndexerError, Result};
use crate::events::{EventKind, PifpEvent};

const MAX_BACKOFF_SECS: u64 = 60;
const INITIAL_BACKOFF_SECS: u64 = 2;
const PIFP_TOPIC_SYMBOLS: &[&str] = &[
    "created",
    "funded",
    "active",
    "verified",
    "expired",
    "cancelled",
    "released",
    "refunded",
    "role_set",
    "role_del",
    "paused",
    "unpaused",
];

// ─────────────────────────────────────────────────────────
// JSON-RPC response shapes
// ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RpcResponse {
    pub result: Option<EventsResult>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct EventsResult {
    pub events: Vec<RawEvent>,
    pub cursor: Option<String>,
    #[serde(rename = "latestLedger")]
    pub latest_ledger: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct RawEvent {
    /// XDR-encoded topic list
    pub topic: Vec<String>,
    /// XDR-encoded event value / data
    pub value: Value,
    #[serde(rename = "contractId")]
    pub contract_id: Option<String>,
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
    pub id: Option<String>,
    pub ledger: Option<u64>,
    #[serde(rename = "ledgerClosedAt")]
    pub ledger_closed_at: Option<String>,
    #[serde(rename = "inSuccessfulContractCall")]
    pub in_successful_contract_call: Option<bool>,
    #[serde(rename = "pagingToken")]
    pub paging_token: Option<String>,
}

// ─────────────────────────────────────────────────────────
// getTransaction response shapes
// ─────────────────────────────────────────────────────────

/// Top-level JSON-RPC envelope for a `getTransaction` call.
#[derive(Debug, Deserialize)]
pub struct TransactionRpcResponse {
    pub result: Option<TransactionResponse>,
    pub error: Option<RpcError>,
}

/// Response payload for the Soroban `getTransaction` RPC method.
///
/// Fields follow the Soroban RPC spec:
/// <https://developers.stellar.org/docs/data/rpc/api-reference/methods/getTransaction>
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct TransactionResponse {
    /// One of `"SUCCESS"`, `"FAILED"`, or `"NOT_FOUND"`.
    pub status: String,
    /// Ledger sequence number in which the transaction was included (absent when NOT_FOUND).
    pub ledger: Option<u64>,
    /// ISO-8601 close time of the ledger that included this transaction.
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    /// Base64-encoded XDR of the `TransactionEnvelope`.
    #[serde(rename = "envelopeXdr")]
    pub envelope_xdr: Option<String>,
    /// Base64-encoded XDR of the `TransactionResult`.
    #[serde(rename = "resultXdr")]
    pub result_xdr: Option<String>,
    /// Base64-encoded XDR of the `TransactionResultMeta` (includes contract return value).
    #[serde(rename = "resultMetaXdr")]
    pub result_meta_xdr: Option<String>,
    /// Diagnostic events emitted during execution (only present when `FAILED`).
    #[serde(rename = "diagnosticEventsXdr")]
    pub diagnostic_events_xdr: Option<Vec<String>>,
}

// ─────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────

/// Fetch a page of events from the RPC.
///
/// * `start_ledger` — the ledger sequence to scan from (inclusive).
/// * `cursor`       — optional opaque pagination cursor from a previous response.
/// * `limit`        — maximum number of events to return.
///
/// Returns `(events, next_cursor, latest_ledger)`.
pub async fn fetch_events(
    client: &Client,
    rpc_url: &str,
    contract_ids: &[String],
    start_ledger: u32,
    cursor: Option<&str>,
    limit: u32,
) -> Result<(Vec<RawEvent>, Option<String>, Option<u64>)> {
    let mut backoff = INITIAL_BACKOFF_SECS;
    let mut use_topic_filter = true;

    loop {
        let params = build_params(
            contract_ids,
            start_ledger,
            cursor,
            limit,
            if use_topic_filter {
                Some(PIFP_TOPIC_SYMBOLS)
            } else {
                None
            },
        );

        let response = client
            .post(rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getEvents",
                "params": params,
            }))
            .send()
            .await;

        match response {
            Err(e) => {
                warn!("RPC request failed (will retry in {backoff}s): {e}");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
                continue;
            }
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    warn!("Rate-limited by RPC (will retry in {backoff}s)");
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
                    continue;
                }

                let body: RpcResponse = resp.json().await?;

                if let Some(err) = body.error {
                    // Some RPC nodes may reject `topics` filter formats.
                    // Retry once with contract-only filtering.
                    if err.code == -32602 && use_topic_filter {
                        warn!(
                            "RPC rejected topic filter (code {}), retrying with contract-only filter",
                            err.code
                        );
                        use_topic_filter = false;
                        continue;
                    }
                    // Code -32600 / -32601 are hard failures; everything else we retry
                    if err.code == -32600 || err.code == -32601 {
                        return Err(IndexerError::EventParse(format!(
                            "RPC hard error {}: {}",
                            err.code, err.message
                        )));
                    }
                    warn!(
                        "RPC soft error (will retry in {backoff}s): {} {}",
                        err.code, err.message
                    );
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
                    continue;
                }

                let result = body.result.ok_or_else(|| {
                    IndexerError::EventParse("Empty result from getEvents".to_string())
                })?;

                debug!(
                    "Fetched {} events (latest_ledger={:?})",
                    result.events.len(),
                    result.latest_ledger
                );

                return Ok((result.events, result.cursor, result.latest_ledger));
            }
        }
    }
}

/// Query the status of a submitted transaction by its hash.
///
/// Calls the Soroban JSON-RPC `getTransaction` method and returns the full
/// [`TransactionResponse`].  Unlike [`fetch_events`] this function does **not**
/// retry on `NOT_FOUND` — the caller is responsible for polling if needed.
///
/// # Errors
/// Returns [`IndexerError::Http`] on network failures and
/// [`IndexerError::EventParse`] when the RPC returns a JSON-RPC error object.
#[allow(dead_code)]
pub async fn get_transaction(
    client: &Client,
    rpc_url: &str,
    hash: &str,
) -> Result<TransactionResponse> {
    let resp = client
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": [hash],
        }))
        .send()
        .await?;

    let body: TransactionRpcResponse = resp.json().await?;

    if let Some(err) = body.error {
        return Err(IndexerError::EventParse(format!(
            "getTransaction RPC error {}: {}",
            err.code, err.message
        )));
    }

    body.result.ok_or_else(|| {
        IndexerError::EventParse("getTransaction returned empty result".to_string())
    })
}

fn build_params(
    contract_ids: &[String],
    start_ledger: u32,
    cursor: Option<&str>,
    limit: u32,
    topic_symbols: Option<&[&str]>,
) -> Value {
    let mut filter = json!({
        "type": "contract",
        "contractIds": contract_ids,
    });
    if let Some(symbols) = topic_symbols {
        let topics: Vec<Value> = symbols
            .iter()
            .map(|s| json!([{"type":"symbol","value": s}]))
            .collect();
        filter["topics"] = Value::Array(topics);
    }

    let mut params = json!({
        "filters": [
            filter
        ],
        "pagination": {
            "limit": limit
        }
    });

    if let Some(cur) = cursor {
        params["pagination"]["cursor"] = json!(cur);
    } else {
        params["startLedger"] = json!(start_ledger);
    }

    params
}

// ─────────────────────────────────────────────────────────
// Event decoding
// ─────────────────────────────────────────────────────────

/// Decode a list of raw RPC events into [`PifpEvent`] structs.
pub fn decode_events(raw: &[RawEvent], contract_ids: &[String]) -> Vec<PifpEvent> {
    raw.iter()
        .filter_map(|e| decode_single(e, contract_ids))
        .collect()
}

fn decode_single(raw: &RawEvent, contract_ids: &[String]) -> Option<PifpEvent> {
    // Extract leading topic symbol to determine event type.
    let first_topic = raw.topic.first()?;
    let first_symbol = extract_symbol(first_topic);
    if !PIFP_TOPIC_SYMBOLS.contains(&first_symbol.as_str()) {
        return None;
    }
    let kind = EventKind::from_topic(&first_symbol);
    if kind == EventKind::Unknown {
        return None;
    }

    // Defense-in-depth: ignore events for contracts outside configured scope.
    if let Some(cid) = raw.contract_id.as_deref() {
        if !contract_ids.iter().any(|c| c == cid) {
            return None;
        }
    }

    let ledger = raw.ledger.unwrap_or(0) as i64;
    let timestamp = raw
        .ledger_closed_at
        .as_deref()
        .and_then(parse_iso_to_unix)
        .unwrap_or(0);

    let project_id = raw.topic.get(1).map(|t| extract_u64_or_raw(t));
    let (actor, amount, extra_data) = decode_data(&raw.value, &kind);

    Some(PifpEvent {
        event_type: kind.as_str().to_string(),
        project_id,
        actor,
        amount,
        extra_data,
        ledger,
        timestamp,
        contract_id: raw
            .contract_id
            .clone()
            .unwrap_or_else(|| contract_ids.first().cloned().unwrap_or_default()),
        tx_hash: raw.tx_hash.clone(),
    })
}

/// Pull apart the JSON `value` blob that Soroban returns for event data.
/// The XDR is decoded by the RPC into a `{"type":…, …}` JSON object.
fn decode_data(
    value: &Value,
    kind: &EventKind,
) -> (Option<String>, Option<String>, Option<String>) {
    match kind {
        EventKind::ProjectCreated => {
            let actor = value
                .get("creator")
                .or_else(|| value.get("address"))
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| find_nested(value, "creator"));
            let amount = value.get("goal").and_then(|v| {
                v.as_str()
                    .map(String::from)
                    .or_else(|| v.as_i64().map(|n| n.to_string()))
            });
            let extra = value.get("token").and_then(value_to_string);
            (actor, amount, extra)
        }
        EventKind::ProjectFunded => {
            let actor = extract_field(value, &["donator", "funder", "address"]);
            let amount = extract_field(value, &["amount"]);
            (actor, amount, None)
        }
        EventKind::ProjectActive | EventKind::ProjectExpired => (None, None, None),
        EventKind::ProjectCancelled => {
            let actor = extract_field(value, &["cancelled_by", "address"]);
            (actor, None, None)
        }
        EventKind::ProjectVerified => {
            let actor = extract_field(value, &["oracle", "verifier", "address"]);
            let extra = extract_field(value, &["proof_hash", "hash", "data"]);
            (actor, None, extra)
        }
        EventKind::FundsReleased => {
            let amount = extract_field(value, &["amount"]);
            let token = extract_field(value, &["token"]);
            (None, amount, token)
        }
        EventKind::DonatorRefunded => {
            let actor = extract_field(value, &["donator", "address"]).or_else(|| {
                value
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(value_to_string)
            });
            let amount = extract_field(value, &["amount"]).or_else(|| {
                value
                    .as_array()
                    .and_then(|arr| arr.get(1))
                    .and_then(value_to_string)
            });
            (actor, amount, None)
        }
        EventKind::RoleSet | EventKind::RoleDel => {
            let actor = value
                .as_str()
                .map(String::from)
                .or_else(|| extract_field(value, &["address", "caller", "by"]));
            (actor, None, None)
        }
        EventKind::ProtocolPaused | EventKind::ProtocolUnpaused => {
            let actor = value
                .as_str()
                .map(String::from)
                .or_else(|| extract_field(value, &["address"]));
            (actor, None, None)
        }
        EventKind::Unknown => (None, None, None),
    }
}

fn extract_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = value.get(key) {
            let s = match v {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => v.as_str().map(String::from),
            };
            if s.is_some() {
                return s;
            }
        }
    }
    None
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Object(obj) => obj
            .get("value")
            .and_then(value_to_string)
            .or_else(|| obj.get("address").and_then(value_to_string)),
        _ => value.as_str().map(String::from),
    }
}

fn find_nested(value: &Value, key: &str) -> Option<String> {
    if let Value::Object(map) = value {
        for (k, v) in map {
            if k == key {
                return v.as_str().map(String::from);
            }
            if let Some(found) = find_nested(v, key) {
                return Some(found);
            }
        }
    }
    None
}

/// Extract a Soroban Symbol from the XDR-decoded topic string.
/// The RPC may return `{"type":"symbol","value":"created"}` or just the raw string.
fn extract_symbol(raw: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        if let Some(s) = v.get("value").and_then(|x| x.as_str()) {
            return s.to_string();
        }
    }
    // Fallback: treat the raw string as the symbol
    raw.to_string()
}

/// Extract the project_id from a topic entry that might be a JSON object or raw number/string.
fn extract_u64_or_raw(raw: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        if let Some(n) = v.get("value").and_then(|x| x.as_u64()) {
            return n.to_string();
        }
        if let Some(s) = v.get("value").and_then(|x| x.as_str()) {
            return s.to_string();
        }
    }
    raw.to_string()
}

/// Parse an ISO-8601 timestamp string into a Unix epoch (seconds).
fn parse_iso_to_unix(s: &str) -> Option<i64> {
    // Simple approach: use chrono
    use chrono::DateTime;
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
}

// ─────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn event_kind_from_topic() {
        assert_eq!(EventKind::from_topic("created"), EventKind::ProjectCreated);
        assert_eq!(EventKind::from_topic("funded"), EventKind::ProjectFunded);
        assert_eq!(
            EventKind::from_topic("verified"),
            EventKind::ProjectVerified
        );
        assert_eq!(EventKind::from_topic("released"), EventKind::FundsReleased);
        assert_eq!(
            EventKind::from_topic("refunded"),
            EventKind::DonatorRefunded
        );
        assert_eq!(EventKind::from_topic("role_set"), EventKind::RoleSet);
        assert_eq!(EventKind::from_topic("role_del"), EventKind::RoleDel);
        assert_eq!(EventKind::from_topic("paused"), EventKind::ProtocolPaused);
        assert_eq!(
            EventKind::from_topic("unpaused"),
            EventKind::ProtocolUnpaused
        );
        assert_eq!(EventKind::from_topic("something_else"), EventKind::Unknown);
    }

    #[test]
    fn event_kind_as_str() {
        assert_eq!(EventKind::ProjectCreated.as_str(), "project_created");
        assert_eq!(EventKind::ProjectFunded.as_str(), "project_funded");
        assert_eq!(EventKind::ProjectVerified.as_str(), "project_verified");
        assert_eq!(EventKind::FundsReleased.as_str(), "funds_released");
        assert_eq!(EventKind::DonatorRefunded.as_str(), "donator_refunded");
        assert_eq!(EventKind::RoleSet.as_str(), "role_set");
        assert_eq!(EventKind::RoleDel.as_str(), "role_del");
    }

    #[test]
    fn extract_symbol_from_json() {
        let raw = r#"{"type":"symbol","value":"funded"}"#;
        assert_eq!(extract_symbol(raw), "funded");
    }

    #[test]
    fn extract_symbol_raw_fallback() {
        assert_eq!(extract_symbol("verified"), "verified");
    }

    #[test]
    fn decode_funded_event() {
        let raw = RawEvent {
            topic: vec![
                r#"{"type":"symbol","value":"funded"}"#.to_string(),
                r#"{"type":"u64","value":"42"}"#.to_string(),
            ],
            value: serde_json::json!({ "donator": "GABC123", "amount": "5000" }),
            contract_id: Some("CONTRACT1".to_string()),
            tx_hash: Some("TX1".to_string()),
            id: None,
            ledger: Some(1000),
            ledger_closed_at: Some("2024-01-01T00:00:00Z".to_string()),
            in_successful_contract_call: Some(true),
            paging_token: None,
        };

        let events = decode_events(&[raw], &["CONTRACT1".to_string()]);
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.event_type, "project_funded");
        assert_eq!(ev.project_id.as_deref(), Some("42"));
        assert_eq!(ev.actor.as_deref(), Some("GABC123"));
        assert_eq!(ev.amount.as_deref(), Some("5000"));
        assert_eq!(ev.ledger, 1000);
    }

    #[test]
    fn decode_role_set_event() {
        let raw = RawEvent {
            topic: vec![
                r#"{"type":"symbol","value":"role_set"}"#.to_string(),
                r#"{"type":"address","value":"GADMIN123"}"#.to_string(),
                r#"{"type":"symbol","value":"admin"}"#.to_string(),
            ],
            value: serde_json::json!("GCALLER"),
            contract_id: Some("CONTRACT1".to_string()),
            tx_hash: Some("TX2".to_string()),
            id: None,
            ledger: Some(1001),
            ledger_closed_at: Some("2024-01-01T00:00:01Z".to_string()),
            in_successful_contract_call: Some(true),
            paging_token: None,
        };

        let events = decode_events(&[raw], &["CONTRACT1".to_string()]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "role_set");
        assert_eq!(events[0].actor.as_deref(), Some("GCALLER"));
    }

    #[test]
    fn decode_refunded_event_tuple_data() {
        let raw = RawEvent {
            topic: vec![
                r#"{"type":"symbol","value":"refunded"}"#.to_string(),
                r#"{"type":"u64","value":"42"}"#.to_string(),
            ],
            value: serde_json::json!(["GDONATOR", "750"]),
            contract_id: Some("CONTRACT1".to_string()),
            tx_hash: Some("TX3".to_string()),
            id: None,
            ledger: Some(1002),
            ledger_closed_at: Some("2024-01-01T00:00:02Z".to_string()),
            in_successful_contract_call: Some(true),
            paging_token: None,
        };

        let events = decode_events(&[raw], &["CONTRACT1".to_string()]);
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.event_type, "donator_refunded");
        assert_eq!(ev.project_id.as_deref(), Some("42"));
        assert_eq!(ev.actor.as_deref(), Some("GDONATOR"));
        assert_eq!(ev.amount.as_deref(), Some("750"));
    }

    #[test]
    fn parse_iso_timestamp() {
        let ts = parse_iso_to_unix("2024-01-01T00:00:00Z").unwrap();
        assert_eq!(ts, 1_704_067_200);
    }

    // ── get_transaction ──────────────────────────────────────────────────────

    #[test]
    fn deserialize_transaction_response_success() {
        let json = serde_json::json!({
            "result": {
                "status": "SUCCESS",
                "ledger": 12345,
                "createdAt": "2024-06-01T12:00:00Z",
                "envelopeXdr": "AAAA",
                "resultXdr": "BBBB",
                "resultMetaXdr": "CCCC",
                "diagnosticEventsXdr": null
            },
            "error": null
        });

        let parsed: TransactionRpcResponse = serde_json::from_value(json).unwrap();
        let tx = parsed.result.unwrap();
        assert_eq!(tx.status, "SUCCESS");
        assert_eq!(tx.ledger, Some(12345));
        assert_eq!(tx.result_xdr.as_deref(), Some("BBBB"));
        assert!(tx.diagnostic_events_xdr.is_none());
    }

    #[test]
    fn deserialize_transaction_response_failed_with_diagnostics() {
        let json = serde_json::json!({
            "result": {
                "status": "FAILED",
                "ledger": 12346,
                "createdAt": "2024-06-01T12:01:00Z",
                "envelopeXdr": "AAAA",
                "resultXdr": "BBBB",
                "resultMetaXdr": "CCCC",
                "diagnosticEventsXdr": ["DIAG1", "DIAG2"]
            },
            "error": null
        });

        let parsed: TransactionRpcResponse = serde_json::from_value(json).unwrap();
        let tx = parsed.result.unwrap();
        assert_eq!(tx.status, "FAILED");
        let diags = tx.diagnostic_events_xdr.unwrap();
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0], "DIAG1");
    }

    #[test]
    fn deserialize_transaction_response_not_found() {
        let json = serde_json::json!({
            "result": {
                "status": "NOT_FOUND"
            },
            "error": null
        });

        let parsed: TransactionRpcResponse = serde_json::from_value(json).unwrap();
        let tx = parsed.result.unwrap();
        assert_eq!(tx.status, "NOT_FOUND");
        assert!(tx.ledger.is_none());
        assert!(tx.result_xdr.is_none());
    }

    #[test]
    fn deserialize_transaction_rpc_error() {
        let json = serde_json::json!({
            "result": null,
            "error": { "code": -32602, "message": "invalid hash" }
        });

        let parsed: TransactionRpcResponse = serde_json::from_value(json).unwrap();
        assert!(parsed.result.is_none());
        let err = parsed.error.unwrap();
        assert_eq!(err.code, -32602);
        assert_eq!(err.message, "invalid hash");
    }

    #[test]
    fn build_params_uses_multiple_contract_ids_and_topics() {
        let ids = vec!["C1".to_string(), "C2".to_string()];
        let params = build_params(&ids, 123, None, 50, Some(PIFP_TOPIC_SYMBOLS));
        let filters = params.get("filters").and_then(|v| v.as_array()).unwrap();
        let filter = &filters[0];
        let contract_ids = filter
            .get("contractIds")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(contract_ids.len(), 2);
        assert_eq!(contract_ids[0], "C1");
        assert_eq!(contract_ids[1], "C2");
        assert!(filter.get("topics").is_some());
    }

    #[test]
    fn decode_skips_events_for_untracked_contract() {
        let raw = RawEvent {
            topic: vec![
                r#"{"type":"symbol","value":"funded"}"#.to_string(),
                r#"{"type":"u64","value":"42"}"#.to_string(),
            ],
            value: serde_json::json!({ "donator": "GABC123", "amount": "5000" }),
            contract_id: Some("OTHER_CONTRACT".to_string()),
            tx_hash: Some("TX1".to_string()),
            id: None,
            ledger: Some(1000),
            ledger_closed_at: Some("2024-01-01T00:00:00Z".to_string()),
            in_successful_contract_call: Some(true),
            paging_token: None,
        };

        let events = decode_events(&[raw], &["CONTRACT1".to_string()]);
        assert!(events.is_empty());
    }

    #[test]
    fn compare_filtered_vs_broad_decode_speed() {
        let tracked = ["CONTRACT1".to_string()];
        let mut events = Vec::new();
        for i in 0..50_000u64 {
            let is_relevant = i % 10 == 0;
            let topic = if is_relevant {
                r#"{"type":"symbol","value":"funded"}"#
            } else {
                r#"{"type":"symbol","value":"not_pifp"}"#
            };
            let contract = if is_relevant { "CONTRACT1" } else { "OTHER" };

            events.push(RawEvent {
                topic: vec![
                    topic.to_string(),
                    format!(r#"{{"type":"u64","value":"{}"}}"#, i % 500),
                ],
                value: serde_json::json!({ "donator": "GABC123", "amount": "5" }),
                contract_id: Some(contract.to_string()),
                tx_hash: None,
                id: None,
                ledger: Some(1),
                ledger_closed_at: Some("2024-01-01T00:00:00Z".to_string()),
                in_successful_contract_call: Some(true),
                paging_token: None,
            });
        }

        let t_filtered = Instant::now();
        let filtered = decode_events(&events, &tracked);
        let filtered_dur = t_filtered.elapsed();

        let t_broad = Instant::now();
        let broad = decode_events_broad_for_benchmark(&events, &tracked[0]);
        let broad_dur = t_broad.elapsed();

        assert_eq!(filtered.len(), 5_000);
        assert_eq!(broad.len(), 50_000);
        println!(
            "decode benchmark: filtered={} in {:?}, broad={} in {:?}",
            filtered.len(),
            filtered_dur,
            broad.len(),
            broad_dur
        );
    }

    fn decode_events_broad_for_benchmark(
        raw: &[RawEvent],
        fallback_contract: &str,
    ) -> Vec<PifpEvent> {
        raw.iter()
            .filter_map(|r| {
                let first_topic = r.topic.first()?;
                let kind = EventKind::from_topic(&extract_symbol(first_topic));
                let project_id = r.topic.get(1).map(|t| extract_u64_or_raw(t));
                let (actor, amount, extra_data) = decode_data(&r.value, &kind);
                Some(PifpEvent {
                    event_type: kind.as_str().to_string(),
                    project_id,
                    actor,
                    amount,
                    extra_data,
                    ledger: r.ledger.unwrap_or(0) as i64,
                    timestamp: r
                        .ledger_closed_at
                        .as_deref()
                        .and_then(parse_iso_to_unix)
                        .unwrap_or(0),
                    contract_id: r
                        .contract_id
                        .clone()
                        .unwrap_or_else(|| fallback_contract.to_string()),
                    tx_hash: r.tx_hash.clone(),
                })
            })
            .collect()
    }
}
