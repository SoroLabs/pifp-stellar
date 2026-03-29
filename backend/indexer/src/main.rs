//! PIFP Event Indexer — entry point.
//!
//! Starts a background indexer task that polls Soroban `getEvents` RPC for
//! PIFP contract events and persists them to SQLite.  Simultaneously
//! exposes a small Axum REST API for frontend / admin consumption.

mod api;
mod cache;
mod config;
mod db;
mod errors;
mod events;
mod indexer;
mod metrics;
mod rpc;
mod webhook;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use reqwest::Client;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use cache::Cache;
use config::Config;
use indexer::IndexerState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise structured logging (RUST_LOG controls verbosity).
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Load optional .env file (ignored if missing).
    let _ = dotenvy::dotenv();

    // Load config from environment.
    let config = Config::from_env().map_err(|e| anyhow::anyhow!("{e}"))?;

    // Set up the SQLite connection pool and run migrations.
    let pool = db::init_pool(&config.database_url).await?;

    // HTTP client shared between the indexer and (future) outbound calls.
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let cache = config
        .redis_url
        .as_deref()
        .and_then(|url| match Cache::new(url) {
            Ok(c) => {
                info!("Redis cache enabled");
                Some(c)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize Redis cache; continuing without cache: {e}");
                None
            }
        });

    // ─── Background indexer ───────────────────────────────
    let indexer_state = Arc::new(IndexerState {
        pool: pool.clone(),
        config: config.clone(),
        client,
        cache: cache.clone(),
    });
    tokio::spawn(indexer::run(indexer_state));

    // ─── REST API ─────────────────────────────────────────
    let api_state = Arc::new(api::ApiState {
        pool,
        cache,
        cache_ttl_top_projects_secs: config.cache_ttl_top_projects_secs,
        cache_ttl_active_projects_count_secs: config.cache_ttl_active_projects_count_secs,
    });

    let app = Router::new()
        .route("/health", get(api::health))
        .route("/events", get(api::get_all_events))
        .route("/projects", get(api::get_projects))
        .route("/projects/:id/history", get(api::get_project_history_paged))
        .route("/projects/top", get(api::get_top_projects))
        .route(
            "/projects/active/count",
            get(api::get_active_projects_count),
        )
        .route("/stats", get(api::get_stats))
        .route("/webhooks", post(api::register_webhook))
        .route("/webhooks", get(api::list_webhooks))
        .route("/admin/quorum", post(api::set_quorum_threshold))
        .route("/projects/:id/vote", post(api::submit_vote))
        .route("/projects/:id/quorum", get(api::get_project_quorum))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(api_state);

    let addr = format!("0.0.0.0:{}", config.api_port);
    info!("API listening on http://{addr}");

    // ─── Metrics server ───────────────────────────────────
    let metrics_addr = format!("0.0.0.0:{}", config.metrics_port);
    info!("Metrics listening on http://{metrics_addr}/metrics");
    let metrics_app = Router::new().route(
        "/metrics",
        get(|| async { metrics::gather_metrics() }),
    );
    let metrics_listener = tokio::net::TcpListener::bind(&metrics_addr).await?;
    tokio::spawn(async move {
        axum::serve(metrics_listener, metrics_app)
            .await
            .expect("metrics server failed");
    });

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
