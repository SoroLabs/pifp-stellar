use std::sync::Arc;

use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tracing::info;

use crate::metrics;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    service: &'static str,
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        service: "pifp-oracle",
    })
}

async fn metrics_handler() -> impl IntoResponse {
    let body = metrics::encode_metrics();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
}

/// Start the health and metrics HTTP server on the given port.
pub async fn serve(
    port: u16,
    bridge_state: Arc<crate::bridge_api::BridgeState>,
    ipfs_state: Arc<crate::ipfs_api::IpfsState>,
    rollup_state: Arc<crate::rollup_api::RollupState>,
    oracle_state: Arc<crate::oracle_api::OracleApiState>,
) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler))
        .merge(crate::bridge_api::router(bridge_state))
        .merge(crate::ipfs_api::router(ipfs_state))
        .merge(crate::rollup_api::router(rollup_state))
        .merge(crate::oracle_api::router(oracle_state));

    let addr = format!("0.0.0.0:{port}");
    info!("Oracle API server listening on http://{addr}");

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::util::ServiceExt;

    fn test_app() -> Router {
        let bridge_state = Arc::new(crate::bridge_api::BridgeState::new());
        let ipfs_state = Arc::new(crate::ipfs_api::IpfsState {
            config: crate::ipfs::IpfsConfig {
                pinata_api_key: None,
                pinata_api_secret: None,
                web3_storage_token: None,
            },
        });
        let rollup_state = Arc::new(crate::rollup_api::RollupState::new(std::time::Duration::from_secs(30)));
        let oracle_state = Arc::new(
            crate::oracle_api::OracleApiState::new(&crate::config::Config {
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
            })
            .expect("oracle state should build in test"),
        );
        Router::new()
            .route("/health", get(health))
            .route("/metrics", get(metrics_handler))
            .merge(crate::bridge_api::router(bridge_state))
            .merge(crate::ipfs_api::router(ipfs_state))
            .merge(crate::rollup_api::router(rollup_state))
            .merge(crate::oracle_api::router(oracle_state))
    }

    #[tokio::test]
    async fn test_health_returns_ok() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_returns_ok() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_response_body() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "pifp-oracle");
        assert!(json["version"].is_string());
    }
}
