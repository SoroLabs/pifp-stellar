//! Soroban RPC client — polls `getEvents` and decodes PIFP events.
//!
//! ## Resilience
//!
//! * [`ProviderManager`] holds a prioritised list of RPC URLs.  On a 5xx,
//!   429, or connection error the current provider is marked unhealthy and
//!   the next one is tried immediately (zero extra latency on the happy path).
//! * A failed provider re-enters the rotation after `cooldown_secs` (default 60 s).
//! * When **all** providers are unhealthy the call falls back to exponential
//!   back-off and logs a critical error.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, error, warn};

use crate::errors::{IndexerError, Result};
use crate::events::{EventKind, PifpEvent};
use crate::metrics;

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
    /// Optional transaction XDR for ML inference
    pub xdr: Option<String>,
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
// Provider health tracking
// ─────────────────────────────────────────────────────────

#[derive(Debug)]
struct ProviderState {
    url: String,
    /// `Some(instant)` means the provider failed at that time and is in cool-down.
    failed_at: Option<Instant>,
}

/// Thread-safe, prioritised list of RPC providers with automatic cool-down.
///
/// Clone is cheap — the inner state is behind an `Arc`.
#[derive(Debug, Clone)]
pub struct ProviderManager {
    providers: Arc<RwLock<Vec<ProviderState>>>,
    cooldown: Duration,
}

impl ProviderManager {
    /// Build from a primary URL plus any number of fallbacks.
    pub fn new(primary: String, fallbacks: Vec<String>, cooldown_secs: u64) -> Self {
        let mut providers: Vec<ProviderState> = std::iter::once(primary)
            .chain(fallbacks)
            .map(|url| ProviderState {
                url,
                failed_at: None,
            })
            .collect();
        // Deduplicate while preserving order.
        let mut seen = std::collections::HashSet::new();
        providers.retain(|p| seen.insert(p.url.clone()));

        Self {
            providers: Arc::new(RwLock::new(providers)),
            cooldown: Duration::from_secs(cooldown_secs),
        }
    }

    /// Return the URL of the first healthy provider, or `None` if all are in cool-down.
    pub fn healthy_url(&self) -> Option<String> {
        let mut providers = self.providers.write().unwrap();
        let now = Instant::now();
        for p in providers.iter_mut() {
            match p.failed_at {
                None => return Some(p.url.clone()),
                Some(t) if now.duration_since(t) >= self.cooldown => {
                    // Cool-down expired — re-enable.
                    p.failed_at = None;
                    return Some(p.url.clone());
                }
                _ => {}
            }
        }
        None
    }

    /// Mark the provider with the given URL as unhealthy.
    pub fn mark_failed(&self, url: &str) {
        let mut providers = self.providers.write().unwrap();
        if let Some(p) = providers.iter_mut().find(|p| p.url == url) {
            if p.failed_at.is_none() {
                warn!("RPC provider marked unhealthy: {url}");
                p.failed_at = Some(Instant::now());
            }
        }
    }
}

// ─────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────

/// Fetch a page of events from the RPC, automatically failing over to the
/// next healthy provider on 5xx / 429 / connection errors.
///
/// * `start_ledger` — the ledger sequence to scan from (inclusive).
/// * `cursor`       — optional opaque pagination cursor from a previous response.
/// * `limit`        — maximum number of events to return.
///
/// Returns `(events, next_cursor, latest_ledger)`.
pub async fn fetch_events(
    client: &Client,
    providers: &ProviderManager,
    contract_ids: &[String],
    start_ledger: u32,
    cursor: Option<&str>,
    limit: u32,
) -> Result<(Vec<RawEvent>, Option<String>, Option<u64>)> {
    let mut backoff = INITIAL_BACKOFF_SECS;
    let mut use_topic_filter = true;

    loop {
        // ── Provider selection ────────────────────────────────────────────────
        let rpc_url = match providers.healthy_url() {
            Some(url) => url,
            None => {
                error!("All RPC providers are unhealthy — retrying in {backoff}s");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
                continue;
            }
        };

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

        let rpc_timer = metrics::RPC_LATENCY.start_timer();

        let send_result = client
            .post(&rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getEvents",
                "params": params,
            }))
            .send()
            .await;

        match send_result {
            Err(e) => {
                rpc_timer.stop_and_record();
                metrics::RPC_ERRORS_TOTAL.inc();
                // Connection / timeout errors — mark provider unhealthy and retry immediately.
                providers.mark_failed(&rpc_url);
                warn!("RPC request failed ({rpc_url}): {e}");
                continue;
            }
            Ok(resp) => {
                let status = resp.status();

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                    rpc_timer.stop_and_record();
                    metrics::RPC_ERRORS_TOTAL.inc();
                    providers.mark_failed(&rpc_url);
                    warn!("RPC provider {rpc_url} returned {status} — switching provider");
                    continue;
                }

                let body: RpcResponse = match resp.json().await {
                    Ok(b) => b,
                    Err(e) => {
                        rpc_timer.stop_and_record();
                        metrics::RPC_ERRORS_TOTAL.inc();
                        return Err(e.into());
                    }
                };
                rpc_timer.stop_and_record();

                if let Some(err) = body.error {
                    if err.code == -32602 && use_topic_filter {
                        warn!(
                            "RPC rejected topic filter (code {}), retrying with contract-only filter",
                            err.code
                        );
                        use_topic_filter = false;
                        continue;
                    }
                    metrics::RPC_ERRORS_TOTAL.inc();
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
                    "Fetched {} events via {} (latest_ledger={:?})",
                    result.events.len(),
                    rpc_url,
                    result.latest_ledger
                );

                return Ok((result.events, result.cursor, result.latest_ledger));
            }
        }
    }
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
            xdr: None,
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
            xdr: None,
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
            xdr: None,
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
            xdr: None,
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
                xdr: None,
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
