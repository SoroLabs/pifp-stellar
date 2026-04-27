//! Pluggable Oracle Aggregator – concurrent multi-provider price fetching with
//! medianizer-based outlier filtering, staleness detection, variance scoring,
//! and a health-score endpoint consumed by the frontend staleness-recovery UI.

use std::{
    cmp::Ordering,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{extract::State, routing::get, Json, Router};
use reqwest::Client;
use serde::Serialize;
use tokio::task::JoinSet;

use crate::config::Config;
use crate::errors::OracleError;

// ── Public response types ─────────────────────────────────────────────────────

/// JSON body returned by `GET /oracle/quote`.
#[derive(Debug, Clone, Serialize)]
pub struct OracleSnapshotResponse {
    pub asset_symbol: String,
    pub quote_symbol: String,
    pub aggregated_price: Option<f64>,
    pub health_score: u8,
    pub status: String,
    /// "green" | "yellow" | "red"
    pub indicator: String,
    pub stale: bool,
    pub high_variance: bool,
    pub variance_pct: f64,
    pub max_sample_age_secs: u64,
    pub updated_at_unix: u64,
    pub provider_count: usize,
    pub contributing_provider_count: usize,
    pub summary: String,
    pub reasons: Vec<String>,
    pub recovery_actions: Vec<String>,
    pub providers: Vec<ProviderObservation>,
}

/// Per-provider observation included in the snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderObservation {
    pub provider: String,
    pub price: Option<f64>,
    pub status: String,
    pub age_secs: u64,
    pub detail: String,
    pub used_in_aggregation: bool,
}

// ── Internal types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CachedSnapshot {
    aggregated_price: Option<f64>,
    variance_pct: f64,
    updated_at_unix: u64,
    providers: Vec<CachedProviderObservation>,
    refresh_error: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedProviderObservation {
    provider: String,
    price: Option<f64>,
    status: String,
    observed_at_unix: u64,
    detail: String,
    used_in_aggregation: bool,
}

#[derive(Debug, Clone)]
struct ProviderSample {
    provider: String,
    price: f64,
    observed_at_unix: u64,
}

#[derive(Debug, Clone)]
enum ProviderFetchResult {
    Sample(ProviderSample),
    Error(CachedProviderObservation),
}

#[derive(Debug, Clone)]
struct OracleRuntime {
    asset_symbol: String,
    quote_symbol: String,
    refresh_interval: Duration,
    max_staleness_secs: u64,
    max_variance_pct: f64,
    providers: Vec<OracleProvider>,
}

/// Shared state for the oracle aggregator API.
#[derive(Debug)]
pub struct OracleApiState {
    client: Client,
    runtime: OracleRuntime,
    cache: Mutex<Option<CachedSnapshot>>,
}

#[derive(Debug, Clone)]
struct OracleProvider {
    name: &'static str,
    url: String,
    parser: ProviderParser,
}

#[derive(Debug, Clone, Copy)]
enum ProviderParser {
    CoinGecko,
    Binance,
    Kraken,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(state: Arc<OracleApiState>) -> Router {
    Router::new()
        .route("/oracle/quote", get(get_oracle_quote))
        .with_state(state)
}

async fn get_oracle_quote(
    State(state): State<Arc<OracleApiState>>,
) -> Json<OracleSnapshotResponse> {
    Json(state.snapshot().await)
}

// ── OracleApiState impl ───────────────────────────────────────────────────────

impl OracleApiState {
    pub fn new(config: &Config) -> Result<Self, OracleError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| {
                OracleError::Network(format!("Failed to build oracle HTTP client: {e}"))
            })?;

        Ok(Self {
            client,
            runtime: OracleRuntime {
                asset_symbol: config.oracle_asset_symbol.clone(),
                quote_symbol: config.oracle_quote_symbol.clone(),
                refresh_interval: Duration::from_secs(config.oracle_refresh_secs),
                max_staleness_secs: config.oracle_max_staleness_secs,
                max_variance_pct: config.oracle_max_variance_pct,
                providers: vec![
                    OracleProvider {
                        name: "CoinGecko",
                        url: config.oracle_coingecko_url.clone(),
                        parser: ProviderParser::CoinGecko,
                    },
                    OracleProvider {
                        name: "Binance",
                        url: config.oracle_binance_url.clone(),
                        parser: ProviderParser::Binance,
                    },
                    OracleProvider {
                        name: "Kraken",
                        url: config.oracle_kraken_url.clone(),
                        parser: ProviderParser::Kraken,
                    },
                ],
            },
            cache: Mutex::new(None),
        })
    }

    async fn snapshot(&self) -> OracleSnapshotResponse {
        let now = now_unix();

        if let Some(cached) = self.cache.lock().unwrap().clone() {
            if now.saturating_sub(cached.updated_at_unix) < self.runtime.refresh_interval.as_secs() {
                return self.materialize(cached, now);
            }
        }

        match self.refresh().await {
            Ok(fresh) => {
                self.cache.lock().unwrap().replace(fresh.clone());
                self.materialize(fresh, now)
            }
            Err(err) => {
                if let Some(mut cached) = self.cache.lock().unwrap().clone() {
                    cached.refresh_error = Some(err);
                    return self.materialize(cached, now);
                }
                self.materialize(
                    CachedSnapshot {
                        aggregated_price: None,
                        variance_pct: 0.0,
                        updated_at_unix: now,
                        providers: self
                            .runtime
                            .providers
                            .iter()
                            .map(|p| CachedProviderObservation {
                                provider: p.name.to_string(),
                                price: None,
                                status: "error".to_string(),
                                observed_at_unix: now,
                                detail: "No provider data available yet".to_string(),
                                used_in_aggregation: false,
                            })
                            .collect(),
                        refresh_error: Some(err),
                    },
                    now,
                )
            }
        }
    }

    async fn refresh(&self) -> Result<CachedSnapshot, String> {
        let mut tasks: JoinSet<ProviderFetchResult> = JoinSet::new();

        for provider in self.runtime.providers.clone() {
            let client = self.client.clone();
            tasks.spawn(async move { fetch_provider_sample(client, provider).await });
        }

        let mut observations: Vec<CachedProviderObservation> = Vec::new();
        let mut samples: Vec<ProviderSample> = Vec::new();

        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(ProviderFetchResult::Sample(s)) => {
                    observations.push(CachedProviderObservation {
                        provider: s.provider.clone(),
                        price: Some(s.price),
                        status: "ok".to_string(),
                        observed_at_unix: s.observed_at_unix,
                        detail: "Provider responded successfully".to_string(),
                        used_in_aggregation: false,
                    });
                    samples.push(s);
                }
                Ok(ProviderFetchResult::Error(e)) => observations.push(e),
                Err(err) => observations.push(CachedProviderObservation {
                    provider: "task".to_string(),
                    price: None,
                    status: "error".to_string(),
                    observed_at_unix: now_unix(),
                    detail: format!("Provider task failed: {err}"),
                    used_in_aggregation: false,
                }),
            }
        }

        if samples.is_empty() {
            return Err("All oracle providers failed; no price could be aggregated".to_string());
        }

        let raw_median = median_price(&samples);
        let tolerance = self.runtime.max_variance_pct / 100.0;
        let mut contributing: Vec<ProviderSample> = Vec::new();

        for s in &samples {
            let dev = relative_deviation(s.price, raw_median);
            let accepted = dev <= tolerance || raw_median == 0.0;

            if accepted {
                contributing.push(s.clone());
            }

            if let Some(obs) = observations.iter_mut().find(|o| o.provider == s.provider) {
                if accepted {
                    obs.used_in_aggregation = true;
                    obs.detail = format!(
                        "Included in medianizer ({:.2}% from pre-filter median)",
                        dev * 100.0
                    );
                } else {
                    obs.status = "outlier".to_string();
                    obs.detail = format!(
                        "Discarded as outlier ({:.2}% from pre-filter median)",
                        dev * 100.0
                    );
                }
            }
        }

        if contributing.is_empty() {
            return Err("Medianizer rejected every provider sample".to_string());
        }

        let aggregated_price = median_price(&contributing);
        let variance_pct = contributing
            .iter()
            .map(|s| relative_deviation(s.price, aggregated_price) * 100.0)
            .fold(0.0_f64, f64::max);

        Ok(CachedSnapshot {
            aggregated_price: Some(aggregated_price),
            variance_pct,
            updated_at_unix: now_unix(),
            providers: observations,
            refresh_error: None,
        })
    }

    fn materialize(&self, cached: CachedSnapshot, now: u64) -> OracleSnapshotResponse {
        let max_sample_age_secs = now.saturating_sub(cached.updated_at_unix);
        let provider_count = cached.providers.len();
        let contributing_provider_count = cached
            .providers
            .iter()
            .filter(|p| p.used_in_aggregation)
            .count();
        let stale = max_sample_age_secs > self.runtime.max_staleness_secs;
        let high_variance = cached.variance_pct > self.runtime.max_variance_pct;

        let coverage_penalty = if provider_count == 0 {
            25.0
        } else {
            (provider_count.saturating_sub(contributing_provider_count)) as f64
                / provider_count as f64
                * 20.0
        };

        let stale_penalty = if stale {
            let overshoot = max_sample_age_secs.saturating_sub(self.runtime.max_staleness_secs) as f64;
            40.0 + (overshoot / self.runtime.max_staleness_secs.max(1) as f64 * 20.0).min(20.0)
        } else {
            (max_sample_age_secs as f64 / self.runtime.max_staleness_secs.max(1) as f64) * 12.0
        };

        let variance_penalty = if high_variance {
            let overshoot = cached.variance_pct - self.runtime.max_variance_pct;
            30.0 + (overshoot / self.runtime.max_variance_pct.max(0.1) * 15.0).min(15.0)
        } else {
            (cached.variance_pct / self.runtime.max_variance_pct.max(0.1)) * 18.0
        };

        let availability_penalty = if cached.aggregated_price.is_some() { 0.0 } else { 55.0 };
        let refresh_penalty = if cached.refresh_error.is_some() { 10.0 } else { 0.0 };

        let raw_score = 100.0
            - stale_penalty
            - variance_penalty
            - coverage_penalty
            - availability_penalty
            - refresh_penalty;
        let health_score = raw_score.clamp(0.0, 100.0).round() as u8;

        let indicator = if health_score >= 80 { "green" } else if health_score >= 55 { "yellow" } else { "red" };
        let status = match indicator { "green" => "healthy", "yellow" => "degraded", _ => "critical" };

        let mut reasons: Vec<String> = Vec::new();
        if cached.aggregated_price.is_none() {
            reasons.push("No aggregated price is currently available".to_string());
        }
        if stale {
            reasons.push(format!(
                "Oracle snapshot is stale at {} seconds old (limit: {}s)",
                max_sample_age_secs, self.runtime.max_staleness_secs
            ));
        }
        if high_variance {
            reasons.push(format!(
                "Provider variance is {:.2}% which exceeds the {:.2}% safety threshold",
                cached.variance_pct, self.runtime.max_variance_pct
            ));
        }
        if contributing_provider_count < provider_count {
            reasons.push(format!(
                "Only {} of {} providers contributed to the final median",
                contributing_provider_count, provider_count
            ));
        }
        if let Some(ref err) = cached.refresh_error {
            reasons.push(format!("Latest refresh attempt failed: {err}"));
        }

        let summary = if cached.aggregated_price.is_none() {
            "Oracle data is unavailable. Protocol actions remain disabled until fresh price feeds recover.".to_string()
        } else if stale || high_variance {
            "Oracle protection mode is active. Critical actions should stay paused until fresh, low-variance data returns.".to_string()
        } else if indicator == "yellow" {
            "Oracle data is available but slightly degraded. Monitor feed freshness before large actions.".to_string()
        } else {
            "Oracle data is fresh and variance is within the safety envelope.".to_string()
        };

        let recovery_actions = vec![
            "Wait for the oracle service to refresh provider responses.".to_string(),
            "Check upstream provider connectivity if the degraded state persists.".to_string(),
            "Retry protocol actions only after the indicator returns to green or the warning is cleared.".to_string(),
        ];

        OracleSnapshotResponse {
            asset_symbol: self.runtime.asset_symbol.clone(),
            quote_symbol: self.runtime.quote_symbol.clone(),
            aggregated_price: cached.aggregated_price.map(round_four_decimals),
            health_score,
            status: status.to_string(),
            indicator: indicator.to_string(),
            stale,
            high_variance,
            variance_pct: round_two_decimals(cached.variance_pct),
            max_sample_age_secs,
            updated_at_unix: cached.updated_at_unix,
            provider_count,
            contributing_provider_count,
            summary,
            reasons,
            recovery_actions,
            providers: cached
                .providers
                .into_iter()
                .map(|p| ProviderObservation {
                    provider: p.provider,
                    price: p.price.map(round_four_decimals),
                    status: p.status,
                    age_secs: now.saturating_sub(p.observed_at_unix),
                    detail: p.detail,
                    used_in_aggregation: p.used_in_aggregation,
                })
                .collect(),
        }
    }
}

// ── Concurrent provider fetching ──────────────────────────────────────────────

async fn fetch_provider_sample(client: Client, provider: OracleProvider) -> ProviderFetchResult {
    let observed_at_unix = now_unix();
    let response = match client.get(&provider.url).send().await {
        Ok(r) => r,
        Err(e) => {
            return ProviderFetchResult::Error(CachedProviderObservation {
                provider: provider.name.to_string(),
                price: None,
                status: "error".to_string(),
                observed_at_unix,
                detail: format!("Request failed: {e}"),
                used_in_aggregation: false,
            })
        }
    };

    let http_status = response.status();
    let body = match response.text().await {
        Ok(b) => b,
        Err(e) => {
            return ProviderFetchResult::Error(CachedProviderObservation {
                provider: provider.name.to_string(),
                price: None,
                status: "error".to_string(),
                observed_at_unix,
                detail: format!("Failed to read response body: {e}"),
                used_in_aggregation: false,
            })
        }
    };

    if !http_status.is_success() {
        return ProviderFetchResult::Error(CachedProviderObservation {
            provider: provider.name.to_string(),
            price: None,
            status: "error".to_string(),
            observed_at_unix,
            detail: format!("HTTP {http_status}: {body}"),
            used_in_aggregation: false,
        });
    }

    match provider.parser.parse_price(&body) {
        Ok(price) => ProviderFetchResult::Sample(ProviderSample {
            provider: provider.name.to_string(),
            price,
            observed_at_unix,
        }),
        Err(err) => ProviderFetchResult::Error(CachedProviderObservation {
            provider: provider.name.to_string(),
            price: None,
            status: "error".to_string(),
            observed_at_unix,
            detail: err,
            used_in_aggregation: false,
        }),
    }
}

// ── Provider-specific JSON parsers ────────────────────────────────────────────

impl ProviderParser {
    fn parse_price(self, body: &str) -> Result<f64, String> {
        let value: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("Invalid JSON payload: {e}"))?;

        let price: Option<f64> = match self {
            ProviderParser::CoinGecko => value
                .get("stellar")
                .and_then(|s| s.get("usd"))
                .and_then(|p| p.as_f64()),
            ProviderParser::Binance => value
                .get("price")
                .and_then(|p| p.as_str())
                .and_then(|s| s.parse::<f64>().ok()),
            ProviderParser::Kraken => value
                .get("result")
                .and_then(|r| r.as_object())
                .and_then(|r| r.values().next())
                .and_then(|e| e.get("c"))
                .and_then(|c| c.get(0))
                .and_then(|p| p.as_str())
                .and_then(|s| s.parse::<f64>().ok()),
        };

        match price {
            Some(p) if p.is_finite() && p > 0.0 => Ok(p),
            _ => Err("Provider payload did not contain a positive finite price".to_string()),
        }
    }
}

// ── Maths helpers ─────────────────────────────────────────────────────────────

fn median_price(samples: &[ProviderSample]) -> f64 {
    let mut prices: Vec<f64> = samples.iter().map(|s| s.price).collect();
    prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = prices.len() / 2;
    if prices.len() % 2 == 0 {
        (prices[mid - 1] + prices[mid]) / 2.0
    } else {
        prices[mid]
    }
}

fn relative_deviation(value: f64, reference: f64) -> f64 {
    if reference == 0.0 { 0.0 } else { (value - reference).abs() / reference.abs() }
}

fn round_two_decimals(v: f64) -> f64 { (v * 100.0).round() / 100.0 }
fn round_four_decimals(v: f64) -> f64 { (v * 10_000.0).round() / 10_000.0 }

fn now_unix() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            horizon_url: "https://horizon-testnet.stellar.org".to_string(),
            contract_id: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM".to_string(),
            oracle_secret_key: "SAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            ipfs_gateway: "https://ipfs.io".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
            timeout_secs: 30,
            sentry_dsn: None,
            metrics_port: 9090,
            oracle_asset_symbol: "XLM".to_string(),
            oracle_quote_symbol: "USD".to_string(),
            oracle_refresh_secs: 15,
            oracle_max_staleness_secs: 90,
            oracle_max_variance_pct: 5.0,
            oracle_coingecko_url: "https://api.coingecko.com/api/v3/simple/price?ids=stellar&vs_currencies=usd".to_string(),
            oracle_binance_url: "https://api.binance.com/api/v3/ticker/price?symbol=XLMUSDT".to_string(),
            oracle_kraken_url: "https://api.kraken.com/0/public/Ticker?pair=XLMUSD".to_string(),
            foreign_rpc_url: None,
            foreign_bridge_address: None,
            node_id: 1,
        }
    }

    fn make_state() -> OracleApiState {
        OracleApiState::new(&test_config()).expect("state should build")
    }

    fn sample(provider: &str, price: f64) -> ProviderSample {
        ProviderSample { provider: provider.to_string(), price, observed_at_unix: now_unix() }
    }

    fn obs(provider: &str, price: Option<f64>, used: bool) -> CachedProviderObservation {
        CachedProviderObservation {
            provider: provider.to_string(),
            price,
            status: if used { "ok" } else { "outlier" }.to_string(),
            observed_at_unix: now_unix(),
            detail: String::new(),
            used_in_aggregation: used,
        }
    }

    #[test]
    fn median_of_three_returns_middle() {
        let samples = vec![sample("A", 0.10), sample("B", 0.12), sample("C", 0.11)];
        assert!((median_price(&samples) - 0.11).abs() < f64::EPSILON);
    }

    #[test]
    fn median_of_two_returns_average() {
        let samples = vec![sample("A", 0.10), sample("B", 0.12)];
        assert!((median_price(&samples) - 0.11).abs() < f64::EPSILON);
    }

    #[test]
    fn relative_deviation_symmetric() {
        assert!((relative_deviation(0.105, 0.10) - 0.05).abs() < 1e-9);
        assert!((relative_deviation(0.095, 0.10) - 0.05).abs() < 1e-9);
    }

    #[test]
    fn relative_deviation_zero_reference() {
        assert_eq!(relative_deviation(0.1, 0.0), 0.0);
    }

    #[test]
    fn medianizer_discards_outliers_above_threshold() {
        let state = make_state();
        let now = now_unix();
        let snap = state.materialize(
            CachedSnapshot {
                aggregated_price: Some(0.103),
                variance_pct: 0.97,
                updated_at_unix: now,
                providers: vec![
                    obs("CoinGecko", Some(0.102), true),
                    obs("Binance", Some(0.103), true),
                    obs("Kraken", Some(0.120), false),
                ],
                refresh_error: None,
            },
            now,
        );
        assert_eq!(snap.contributing_provider_count, 2);
        assert_eq!(snap.provider_count, 3);
        assert_eq!(snap.indicator, "green");
    }

    #[test]
    fn materialized_snapshot_flags_staleness_and_variance() {
        let state = make_state();
        let now = now_unix();
        let snap = state.materialize(
            CachedSnapshot {
                aggregated_price: Some(0.101),
                variance_pct: 6.2,
                updated_at_unix: now - 180,
                providers: vec![obs("CoinGecko", Some(0.101), true)],
                refresh_error: Some("Binance timeout".to_string()),
            },
            now,
        );
        assert!(snap.stale);
        assert!(snap.high_variance);
        assert_eq!(snap.indicator, "red");
        assert!(snap.reasons.iter().any(|r| r.contains("Latest refresh attempt failed")));
    }

    #[test]
    fn missing_price_is_red() {
        let state = make_state();
        let now = now_unix();
        let snap = state.materialize(
            CachedSnapshot {
                aggregated_price: None,
                variance_pct: 0.0,
                updated_at_unix: now,
                providers: vec![obs("Binance", None, false)],
                refresh_error: Some("timeout".to_string()),
            },
            now,
        );
        assert_eq!(snap.indicator, "red");
    }

    #[test]
    fn coingecko_parser_extracts_price() {
        let body = r#"{"stellar":{"usd":0.1025}}"#;
        let price = ProviderParser::CoinGecko.parse_price(body).unwrap();
        assert!((price - 0.1025).abs() < 1e-10);
    }

    #[test]
    fn binance_parser_extracts_price() {
        let body = r#"{"symbol":"XLMUSDT","price":"0.1019"}"#;
        let price = ProviderParser::Binance.parse_price(body).unwrap();
        assert!((price - 0.1019).abs() < 1e-10);
    }

    #[test]
    fn kraken_parser_extracts_price() {
        let body = r#"{"result":{"XXLMZUSD":{"c":["0.1018","1"]}}}"#;
        let price = ProviderParser::Kraken.parse_price(body).unwrap();
        assert!((price - 0.1018).abs() < 1e-10);
    }

    #[test]
    fn parser_rejects_invalid_json() {
        assert!(ProviderParser::CoinGecko.parse_price("not-json").is_err());
    }

    #[test]
    fn parser_rejects_zero_price() {
        assert!(ProviderParser::CoinGecko.parse_price(r#"{"stellar":{"usd":0}}"#).is_err());
    }

    #[tokio::test]
    async fn refresh_aggregates_mocked_provider_responses() {
        let mut server = mockito::Server::new_async().await;
        let _cg = server.mock("GET", "/coingecko").with_status(200)
            .with_body(r#"{"stellar":{"usd":0.102}}"#).create_async().await;
        let _bn = server.mock("GET", "/binance").with_status(200)
            .with_body(r#"{"price":"0.101"}"#).create_async().await;
        let _kr = server.mock("GET", "/kraken").with_status(200)
            .with_body(r#"{"result":{"XXLMZUSD":{"c":["0.115","1"]}}}"#).create_async().await;

        let mut config = test_config();
        config.oracle_coingecko_url = format!("{}/coingecko", server.url());
        config.oracle_binance_url = format!("{}/binance", server.url());
        config.oracle_kraken_url = format!("{}/kraken", server.url());

        let state = OracleApiState::new(&config).expect("state should build");
        let cached = state.refresh().await.expect("refresh should succeed");

        assert!(cached.aggregated_price.is_some());
        let contrib = cached.providers.iter().filter(|p| p.used_in_aggregation).count();
        assert_eq!(contrib, 2);
        assert!(cached.providers.iter().any(|p| p.status == "outlier"));
    }

    #[tokio::test]
    async fn refresh_returns_err_when_all_providers_fail() {
        let mut server = mockito::Server::new_async().await;
        let _cg = server.mock("GET", "/coingecko").with_status(500).create_async().await;
        let _bn = server.mock("GET", "/binance").with_status(500).create_async().await;
        let _kr = server.mock("GET", "/kraken").with_status(500).create_async().await;

        let mut config = test_config();
        config.oracle_coingecko_url = format!("{}/coingecko", server.url());
        config.oracle_binance_url = format!("{}/binance", server.url());
        config.oracle_kraken_url = format!("{}/kraken", server.url());

        let state = OracleApiState::new(&config).expect("state should build");
        let result = state.refresh().await;
        assert!(result.is_err());
    }
}
