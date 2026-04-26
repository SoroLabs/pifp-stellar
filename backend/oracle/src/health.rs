use std::sync::Arc;

use axum::{
    extract::State,
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

#[derive(Clone)]
pub struct ServerState {
    pub bridge: Arc<crate::bridge_api::BridgeState>,
    pub ipfs: Arc<crate::ipfs_api::IpfsState>,
}

async fn health(State(_state): State<Arc<ServerState>>) -> impl IntoResponse {
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
) -> anyhow::Result<()> {
    let state = Arc::new(ServerState {
        bridge: bridge_state.clone(),
        ipfs: ipfs_state.clone(),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler))
        .nest("/api", crate::bridge_api::router(bridge_state))
        .nest("/api", crate::ipfs_api::router(ipfs_state))
        .with_state(state);

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
        let state = Arc::new(ServerState {
            bridge: bridge_state.clone(),
            ipfs: ipfs_state.clone(),
        });
        Router::new()
            .route("/health", get(health))
            .route("/metrics", get(metrics_handler))
            .nest("/api", crate::bridge_api::router(bridge_state))
            .nest("/api", crate::ipfs_api::router(ipfs_state))
            .with_state(state)
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
