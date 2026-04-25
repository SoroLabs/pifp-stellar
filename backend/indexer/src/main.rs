//! PIFP Event Indexer — entry point.
//!
//! Starts a background indexer task that polls Soroban `getEvents` RPC for
//! PIFP contract events and persists them to SQLite.  Simultaneously
//! exposes a small Axum REST API for frontend / admin consumption.

pub(crate) mod api;
pub(crate) mod atomic_swap;
pub(crate) mod cache;
pub(crate) mod config;
pub(crate) mod db;
pub(crate) mod errors;
pub(crate) mod events;
pub(crate) mod indexer;
pub(crate) mod metrics;
pub(crate) mod middleware;
pub(crate) mod ml_pipeline;
pub(crate) mod p2p_topology;
pub(crate) mod profiles;
pub(crate) mod rate_limit;
pub(crate) mod rpc;
pub(crate) mod webhook;

#[cfg(test)]
mod auth_test;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    routing::{get, post},
    Router,
};
use reqwest::Client;
use sentry::{self, protocol::Event};
use sysinfo::System;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

use cache::Cache;
use config::Config;
use indexer::IndexerState;
use rate_limit::{AdaptiveStore, RateLimitLayer, RateLimiterStore};
use rpc::ProviderManager;

fn redact_sensitive_data(mut event: Event<'static>) -> Event<'static> {
    event.request = None;
    event.user = None;
    event.extra.clear();
    event.tags.retain(|k, _| {
        let key = k.to_ascii_lowercase();
        !key.contains("auth") && !key.contains("token") && !key.contains("password")
    });
    event
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load optional .env file (ignored if missing).
    let _ = dotenvy::dotenv();

    // Load config from environment.
    let config = Config::from_env().map_err(|e| anyhow::anyhow!("{e}"))?;

    // Initialise Sentry if configured.
    let _sentry_guard = config.sentry_dsn.as_deref().map(|dsn| {
        info!("Sentry error tracking enabled");
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 1.0,
                before_send: Some(std::sync::Arc::new(|event: Event<'static>| {
                    Some(redact_sensitive_data(event))
                })),
                ..Default::default()
            },
        ))
    });

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .init();

    // Set up the SQLite connection pool and run migrations.
    let pool = db::init_pool(&config.database_url).await?;

    // HTTP client shared between the indexer and (future) outbound calls.
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    let cache = config
        .redis_url
        .as_deref()
        .and_then(|url| match Cache::new(url) {
            Ok(c) => {
                info!("Redis cache enabled");
                Some(c)
            }
            Err(e) => {
                warn!("Failed to initialize Redis cache; continuing without cache: {e}");
                None
            }
        });

    // ─── Adaptive Rate Limiter Store ──────────────────────
    let rate_limit_store = Arc::new(AdaptiveStore::new(
        config
            .api_rate_limit
            .unwrap_or(rate_limit::DEFAULT_REQUESTS_PER_MINUTE),
    ));

    // ─── System Metrics Monitor ───────────────────────────
    let rate_limit_store_clone = Arc::clone(&rate_limit_store);
    tokio::spawn(async move {
        let mut sys = System::new_all();
        loop {
            sys.refresh_cpu_all();
            sys.refresh_memory();

            let cpu_usage = sys.global_cpu_usage() as f64 / 100.0;
            let mem_usage = sys.used_memory() as f64 / sys.total_memory() as f64;

            rate_limit_store_clone.update_metrics(cpu_usage, mem_usage);

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // ─── ML Pipeline ─────────────────────────────────────
    let ml_pipeline = Arc::new(ml_pipeline::MLPipeline::new(None)?);

    // ─── Background indexer ───────────────────────────────
    let providers = ProviderManager::new(
        config.rpc_url.clone(),
        config.rpc_fallback_urls.clone(),
        config.rpc_cooldown_secs,
    );
    let indexer_state = Arc::new(IndexerState {
        pool: pool.clone(),
        config: config.clone(),
        client,
        cache: cache.clone(),
        providers,
        ml_pipeline,
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
        .route("/projects/search", get(api::search_projects))
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
        .route("/profiles/:address", get(api::get_profile))
        .route(
            "/profiles/:address",
            axum::routing::put(api::upsert_profile),
        )
        .route(
            "/profiles/:address",
            axum::routing::delete(api::delete_profile),
        )
        .layer(RateLimitLayer::new(rate_limit_store))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(api_state);

    let addr = format!("0.0.0.0:{}", config.api_port);
    info!("API listening on http://{addr}");

    // ─── Metrics server ───────────────────────────────────
    let metrics_addr = format!("0.0.0.0:{}", config.metrics_port);
    info!("Metrics listening on http://{metrics_addr}/metrics");
    let metrics_app = Router::new().route("/metrics", get(|| async { metrics::gather_metrics() }));
    let metrics_listener = TcpListener::bind(&metrics_addr).await?;
    tokio::spawn(async move {
        axum::serve(metrics_listener, metrics_app)
            .await
            .expect("metrics server failed");
    });

    // ─── Optimized TCP Acceptor ───────────────────────────
    let listener = TcpListener::bind(&addr).await?;

    // Using a custom loop for zero-copy handoffs if needed in the future,
    // for now axum::serve with a standard listener is highly concurrent.
    // Issue #269 asks for zero-copy socket handoffs, which usually means
    // using something like `tokio-util`'s `Framed` or direct syscalls.
    // Standard `axum::serve` uses `tokio::net::TcpListener` which is already
    // quite efficient. For true zero-copy we'd need a more complex setup.

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
