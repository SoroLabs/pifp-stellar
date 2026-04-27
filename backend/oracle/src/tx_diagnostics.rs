use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct TxDiagnostics {
    pub tx_hash: String,
    pub status: String,
    pub failure_point: String,
    pub soroban_error_code: Option<i64>,
    pub protocol_issue: String,
    pub recovery_steps: Vec<String>,
    pub result_xdr: Option<String>,
    pub diagnostic_events: Vec<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct TxDiagnosticsStore {
    inner: Arc<RwLock<HashMap<String, TxDiagnostics>>>,
}

impl TxDiagnosticsStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn upsert(&self, diagnostics: TxDiagnostics) {
        let mut map = self.inner.write().expect("tx diagnostics store poisoned");
        map.insert(diagnostics.tx_hash.clone(), diagnostics);
    }

    pub fn get(&self, hash: &str) -> Option<TxDiagnostics> {
        let map = self.inner.read().expect("tx diagnostics store poisoned");
        map.get(hash).cloned()
    }
}

pub fn build_failed_tx_diagnostics(tx_hash: &str, tx_result: &Value, failure_point: &str) -> TxDiagnostics {
    let result = tx_result.get("result").unwrap_or(tx_result);
    let status = read_string_field(result, &["status"]).unwrap_or_else(|| "FAILED".to_string());
    let result_xdr = read_string_field(result, &["resultXdr", "result_xdr", "resultMetaXdr", "result_meta_xdr"]);

    let diagnostic_events = read_diagnostic_events(result);
    let soroban_error_code = extract_error_code(result, &diagnostic_events);
    let (protocol_issue, recovery_steps) = map_error_code(soroban_error_code);

    TxDiagnostics {
        tx_hash: tx_hash.to_string(),
        status,
        failure_point: failure_point.to_string(),
        soroban_error_code,
        protocol_issue,
        recovery_steps,
        result_xdr,
        diagnostic_events,
    }
}

pub fn map_error_code(code: Option<i64>) -> (String, Vec<String>) {
    match code {
        Some(1) => (
            "Project not found".to_string(),
            vec![
                "Confirm the project ID exists before submitting verification.".to_string(),
                "Refresh project data and retry with a valid identifier.".to_string(),
            ],
        ),
        Some(3) => (
            "Project already completed".to_string(),
            vec![
                "Do not re-run verification for already released milestones.".to_string(),
                "Refresh project status and continue with the next pending project.".to_string(),
            ],
        ),
        Some(6) => (
            "Oracle not authorized".to_string(),
            vec![
                "Grant oracle role for the submitting address on the protocol contract.".to_string(),
                "Retry after role assignment transaction is confirmed.".to_string(),
            ],
        ),
        Some(14) => (
            "Project has expired".to_string(),
            vec![
                "Review expiration and grace-period policy for the project.".to_string(),
                "Trigger the expired-project recovery path instead of verify_and_release.".to_string(),
            ],
        ),
        Some(16) => (
            "Proof hash mismatch".to_string(),
            vec![
                "Recompute the proof hash from the latest proof artifact.".to_string(),
                "Ensure the submitted CID matches the hash committed on-chain.".to_string(),
            ],
        ),
        Some(other) => (
            format!("Unknown Soroban contract error code {other}"),
            vec![
                "Inspect diagnostic events and result_xdr for contract-specific failure details.".to_string(),
                "Retry only after identifying and resolving the underlying contract precondition.".to_string(),
            ],
        ),
        None => (
            "Unable to determine exact contract error code".to_string(),
            vec![
                "Inspect diagnostic_events and result_xdr for execution trace details.".to_string(),
                "Re-run simulation to reproduce and isolate the failing cross-contract hop.".to_string(),
            ],
        ),
    }
}

fn read_string_field(root: &Value, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .filter_map(|key| root.get(*key))
        .find_map(|value| value.as_str().map(ToOwned::to_owned))
}

fn read_diagnostic_events(root: &Value) -> Vec<Value> {
    if let Some(Value::Array(items)) = root.get("diagnosticEvents") {
        return items.clone();
    }
    if let Some(Value::Array(items)) = root.get("diagnostic_events") {
        return items.clone();
    }
    if let Some(Value::Array(items)) = root.get("diagnosticEventsXdr") {
        return items.clone();
    }
    if let Some(Value::Array(items)) = root.get("diagnostic_events_xdr") {
        return items.clone();
    }
    Vec::new()
}

fn extract_error_code(root: &Value, diagnostic_events: &[Value]) -> Option<i64> {
    read_number_field(root, &["code", "errorCode", "contractErrorCode"])
        .or_else(|| search_nested_for_code(root))
        .or_else(|| scan_values_for_tagged_code(root))
        .or_else(|| {
            diagnostic_events
                .iter()
                .find_map(search_nested_for_code)
                .or_else(|| diagnostic_events.iter().find_map(scan_values_for_tagged_code))
        })
}

fn read_number_field(root: &Value, candidates: &[&str]) -> Option<i64> {
    for key in candidates {
        if let Some(value) = root.get(*key) {
            if let Some(v) = value.as_i64() {
                return Some(v);
            }
            if let Some(s) = value.as_str() {
                if let Ok(parsed) = s.parse::<i64>() {
                    return Some(parsed);
                }
                if let Some(tagged) = parse_tagged_contract_error(s) {
                    return Some(tagged);
                }
            }
        }
    }
    None
}

fn search_nested_for_code(value: &Value) -> Option<i64> {
    match value {
        Value::Object(map) => {
            if let Some(code) = read_number_field(value, &["code", "errorCode", "contractErrorCode"]) {
                return Some(code);
            }
            map.values().find_map(search_nested_for_code)
        }
        Value::Array(items) => items.iter().find_map(search_nested_for_code),
        _ => None,
    }
}

fn scan_values_for_tagged_code(value: &Value) -> Option<i64> {
    match value {
        Value::String(s) => parse_tagged_contract_error(s),
        Value::Object(map) => map.values().find_map(scan_values_for_tagged_code),
        Value::Array(items) => items.iter().find_map(scan_values_for_tagged_code),
        _ => None,
    }
}

fn parse_tagged_contract_error(input: &str) -> Option<i64> {
    let marker = "Error(Contract, #";
    let start = input.find(marker)? + marker.len();
    let tail = &input[start..];
    let end = tail.find(')')?;
    tail[..end].trim().parse::<i64>().ok()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{build_failed_tx_diagnostics, map_error_code};

    #[test]
    fn map_known_error_code_has_actionable_steps() {
        let (issue, steps) = map_error_code(Some(16));
        assert_eq!(issue, "Proof hash mismatch");
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn diagnostics_parser_extracts_error_code_and_xdr() {
        let payload = json!({
            "result": {
                "status": "FAILED",
                "resultXdr": "AAAAFAKE",
                "diagnosticEvents": [
                    {"event": "something"},
                    {"message": "Error(Contract, #6)"}
                ]
            }
        });

        let diagnostics = build_failed_tx_diagnostics("abc123", &payload, "getTransaction");

        assert_eq!(diagnostics.soroban_error_code, Some(6));
        assert_eq!(diagnostics.result_xdr.as_deref(), Some("AAAAFAKE"));
        assert_eq!(diagnostics.failure_point, "getTransaction");
    }
}
