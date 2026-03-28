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
    pub start_time: std::time::Instant,
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
pub async fn serve(port: u16) -> anyhow::Result<()> {
    let state = Arc::new(ServerState {
        start_time: std::time::Instant::now(),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    info!("Health server listening on http://{addr}");

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
        let state = Arc::new(ServerState {
            start_time: std::time::Instant::now(),
        });
        Router::new()
            .route("/health", get(health))
            .route("/metrics", get(metrics_handler))
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
