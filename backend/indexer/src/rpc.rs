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
    contract_id: &str,
    start_ledger: u32,
    cursor: Option<&str>,
    limit: u32,
) -> Result<(Vec<RawEvent>, Option<String>, Option<u64>)> {
    let mut backoff = INITIAL_BACKOFF_SECS;

    loop {
        let params = build_params(contract_id, start_ledger, cursor, limit);

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

fn build_params(contract_id: &str, start_ledger: u32, cursor: Option<&str>, limit: u32) -> Value {
    let mut params = json!({
        "filters": [
            {
                "type": "contract",
                "contractIds": [contract_id]
            }
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
pub fn decode_events(raw: &[RawEvent], contract_id: &str) -> Vec<PifpEvent> {
    raw.iter()
        .filter_map(|e| decode_single(e, contract_id))
        .collect()
}

fn decode_single(raw: &RawEvent, contract_id: &str) -> Option<PifpEvent> {
    // Extract leading topic symbol to determine event type.
    let first_topic = raw.topic.first()?;
    let kind = EventKind::from_topic(&extract_symbol(first_topic));

    let ledger = raw.ledger.unwrap_or(0) as i64;
    let timestamp = raw
        .ledger_closed_at
        .as_deref()
        .and_then(parse_iso_to_unix)
        .unwrap_or(0);

    let project_id = raw.topic.get(1).map(|t| extract_u64_or_raw(t));

    let (actor, amount) = decode_data(&raw.value, &kind);

    Some(PifpEvent {
        event_type: kind.as_str().to_string(),
        project_id,
        actor,
        amount,
        ledger,
        timestamp,
        contract_id: raw
            .contract_id
            .clone()
            .unwrap_or_else(|| contract_id.to_string()),
        tx_hash: raw.tx_hash.clone(),
    })
}

/// Pull apart the JSON `value` blob that Soroban returns for event data.
/// The XDR is decoded by the RPC into a `{"type":…, …}` JSON object.
fn decode_data(value: &Value, kind: &EventKind) -> (Option<String>, Option<String>) {
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
            (actor, amount)
        }
        EventKind::ProjectFunded => {
            let actor = extract_field(value, &["donator", "funder", "address"]);
            let amount = extract_field(value, &["amount"]);
            (actor, amount)
        }
        EventKind::ProjectVerified => {
            let actor = extract_field(value, &["oracle", "verifier", "address"]);
            (actor, None)
        }
        EventKind::FundsReleased => {
            let amount = extract_field(value, &["amount"]);
            (None, amount)
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
            (actor, amount)
        }
        EventKind::RoleSet | EventKind::RoleDel => {
            // For role events the "data" is typically the caller address
            let actor = value
                .as_str()
                .map(String::from)
                .or_else(|| extract_field(value, &["address", "caller", "by"]));
            (actor, None)
        }
        EventKind::ProtocolPaused | EventKind::ProtocolUnpaused => {
            let actor = value
                .as_str()
                .map(String::from)
                .or_else(|| extract_field(value, &["address"]));
            (actor, None)
        }
        EventKind::Unknown => (None, None),
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

        let events = decode_events(&[raw], "CONTRACT1");
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

        let events = decode_events(&[raw], "CONTRACT1");
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

        let events = decode_events(&[raw], "CONTRACT1");
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
}
