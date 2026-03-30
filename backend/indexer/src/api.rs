//! Axum REST API handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::info;

use crate::cache::Cache;
use crate::db;
use crate::events::EventRecord;
use crate::middleware::auth::verify_profile_signature;
use crate::profiles::{self, ProfileUpdate};

#[derive(Clone)]
pub struct ApiState {
    pub pool: SqlitePool,
    pub cache: Option<Cache>,
    pub cache_ttl_top_projects_secs: u64,
    pub cache_ttl_active_projects_count_secs: u64,
}

// ─────────────────────────────────────────────────────────
// Response shapes
// ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct EventsResponse {
    pub project_id: String,
    pub count: usize,
    pub events: Vec<EventRecord>,
}

#[derive(Serialize)]
pub struct AllEventsResponse {
    pub count: usize,
    pub events: Vec<EventRecord>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Deserialize)]
pub struct ProjectQuery {
    pub status: Option<String>,
    pub creator: Option<String>,
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
}
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct ProjectsResponse {
    pub count: usize,
    pub projects: Vec<db::ProjectRecord>,
}

#[derive(Deserialize)]
pub struct VoteRequest {
    pub oracle: String,
    pub proof_hash: String,
}

#[derive(Deserialize)]
pub struct ThresholdRequest {
    pub threshold: u32,
}

#[derive(Serialize)]
pub struct VoteResponse {
    pub accepted: bool,
    pub message: String,
}

/// Signed profile upsert request.
///
/// The client must sign the canonical message `"pifp-profile:{address}"` with
/// the Ed25519 private key corresponding to `address` and provide the
/// base64-encoded signature in `signature`.
#[derive(Deserialize)]
pub struct ProfileRequest {
    pub address: String,
    pub signature: String,
    #[serde(flatten)]
    pub update: ProfileUpdate,
}

#[derive(Deserialize)]
pub struct TopProjectsQuery {
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct TopProjectsResponse {
    pub count: usize,
    pub projects: Vec<db::TopProject>,
}

#[derive(Serialize, Deserialize)]
pub struct ActiveProjectsCountResponse {
    pub count: i64,
}

#[derive(Serialize, Deserialize)]
pub struct StatsResponse {
    pub total_projects: i64,
    pub total_tvl: String,
    pub total_donors: i64,
    pub completed_projects: i64,
    pub failed_projects: i64,
    pub success_rate: f64,
}

#[derive(Deserialize)]
pub struct RegisterWebhookRequest {
    pub url: String,
    pub secret: String,
    pub event_types: Vec<String>,
}

// ─────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────

/// `GET /health`
pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// `GET /projects/:id/history`
///
/// Returns project event history with pagination.
pub async fn get_project_history_paged(
    State(state): State<Arc<ApiState>>,
    Path(project_id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    match db::get_project_history(&state.pool, &project_id, limit, offset).await {
        Ok(events) => {
            let count = events.len();
            (
                StatusCode::OK,
                Json(serde_json::json!(EventsResponse {
                    project_id,
                    count,
                    events,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /projects`
///
/// Returns all projects matching optional filters (status, creator, category), with pagination.
pub async fn get_projects(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ProjectQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);
    let categories = query.category.as_deref().map(|csv| {
        csv.split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    });

    match db::list_projects(
        &state.pool,
        query.status,
        query.creator,
        categories,
        limit,
        offset,
    )
    .await
    {
        Ok(projects) => {
            let count = projects.len();
            (
                StatusCode::OK,
                Json(serde_json::json!(ProjectsResponse { count, projects })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /projects/search?q=...&limit=20&page=0`
///
/// Full-text search over project title and description.
/// Returns an empty array (not an error) for blank queries or no matches.
pub async fn search_projects(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = query.q.unwrap_or_default();
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let page = query.page.unwrap_or(0).max(0);
    let offset = page * limit;

    match db::search_projects(&state.pool, &q, limit, offset).await {
        Ok(projects) => {
            let count = projects.len();
            (
                StatusCode::OK,
                Json(serde_json::json!(ProjectsResponse { count, projects })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /events`
///
/// Returns all indexed events across all projects.
pub async fn get_all_events(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match db::get_all_events(&state.pool).await {
        Ok(events) => {
            let count = events.len();
            (
                StatusCode::OK,
                Json(serde_json::json!(AllEventsResponse { count, events })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `POST /admin/quorum`
///
/// Updates the global quorum threshold.
pub async fn set_quorum_threshold(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<ThresholdRequest>,
) -> impl IntoResponse {
    match db::set_quorum_threshold(&state.pool, payload.threshold).await {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "updated" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `POST /projects/:id/vote`
///
/// Submits an oracle vote for a project.
pub async fn submit_vote(
    State(state): State<Arc<ApiState>>,
    Path(project_id): Path<String>,
    Json(payload): Json<VoteRequest>,
) -> impl IntoResponse {
    match db::record_vote(
        &state.pool,
        &project_id,
        &payload.oracle,
        &payload.proof_hash,
    )
    .await
    {
        Ok(accepted) => {
            let (status, message) = if accepted {
                (StatusCode::CREATED, "Vote recorded")
            } else {
                (StatusCode::OK, "Duplicate vote ignored")
            };
            (
                status,
                Json(VoteResponse {
                    accepted,
                    message: message.to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /projects/:id/quorum`
///
/// Returns current quorum status for a project.
pub async fn get_project_quorum(
    State(state): State<Arc<ApiState>>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match db::get_quorum_status(&state.pool, &project_id).await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /profiles/:address`
pub async fn get_profile(
    State(state): State<Arc<ApiState>>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match profiles::get_profile(&state.pool, &address).await {
        Ok(Some(profile)) => (StatusCode::OK, Json(serde_json::json!(profile))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!(ErrorResponse {
                error: "Profile not found".to_string()
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `PUT /profiles/:address`
///
/// Upserts a profile. Requires a valid Ed25519 signature over
/// `"pifp-profile:{address}"` from the address owner.
pub async fn upsert_profile(
    State(state): State<Arc<ApiState>>,
    Path(address): Path<String>,
    Json(payload): Json<ProfileRequest>,
) -> impl IntoResponse {
    if payload.address != address {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!(ErrorResponse {
                error: "Address mismatch".to_string()
            })),
        )
            .into_response();
    }

    if !verify_profile_signature(&address, &payload.signature).is_ok() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!(ErrorResponse {
                error: "Invalid signature".to_string()
            })),
        )
            .into_response();
    }

    match profiles::upsert_profile(&state.pool, &address, &payload.update).await {
        Ok(profile) => (StatusCode::OK, Json(serde_json::json!(profile))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /projects/top?limit=10`
///
/// Returns the top funded projects, optionally cached in Redis.
pub async fn get_top_projects(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<TopProjectsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(10).clamp(1, 100);
    let mut cache_key = None;

    if let Some(cache) = &state.cache {
        let version = cache.get_version().await;
        let key = format!("indexer:top_projects:v{version}:limit:{limit}");
        if let Some(cached) = cache.get_json::<TopProjectsResponse>(&key).await {
            info!("cache hit: endpoint=top_projects key={key}");
            return (StatusCode::OK, Json(cached)).into_response();
        }
        info!("cache miss: endpoint=top_projects key={key}");
        cache_key = Some(key);
    }

    match db::get_top_projects(&state.pool, limit).await {
        Ok(projects) => {
            let payload = TopProjectsResponse {
                count: projects.len(),
                projects,
            };
            if let (Some(cache), Some(key)) = (&state.cache, cache_key.as_deref()) {
                cache
                    .set_json(key, &payload, state.cache_ttl_top_projects_secs)
                    .await;
            }
            (StatusCode::OK, Json(payload)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `DELETE /profiles/:address`
///
/// Deletes a profile. Requires a valid Ed25519 signature over
/// `"pifp-profile:{address}"` from the address owner.
pub async fn delete_profile(
    State(state): State<Arc<ApiState>>,
    Path(address): Path<String>,
    Json(payload): Json<ProfileRequest>,
) -> impl IntoResponse {
    if payload.address != address {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!(ErrorResponse {
                error: "Address mismatch".to_string()
            })),
        )
            .into_response();
    }

    if !verify_profile_signature(&address, &payload.signature).is_ok() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!(ErrorResponse {
                error: "Invalid signature".to_string()
            })),
        )
            .into_response();
    }

    match profiles::delete_profile(&state.pool, &address).await {
        Ok(true) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "deleted" })),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!(ErrorResponse {
                error: "Profile not found".to_string()
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /projects/active/count`
///
/// Returns the current active projects count, optionally cached in Redis.
pub async fn get_active_projects_count(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let mut cache_key = None;
    if let Some(cache) = &state.cache {
        let version = cache.get_version().await;
        let key = format!("indexer:active_projects_count:v{version}");
        if let Some(cached) = cache.get_json::<ActiveProjectsCountResponse>(&key).await {
            info!("cache hit: endpoint=active_projects_count key={key}");
            return (StatusCode::OK, Json(cached)).into_response();
        }
        info!("cache miss: endpoint=active_projects_count key={key}");
        cache_key = Some(key);
    }

    match db::get_active_projects_count(&state.pool).await {
        Ok(count) => {
            let payload = ActiveProjectsCountResponse { count };
            if let (Some(cache), Some(key)) = (&state.cache, cache_key.as_deref()) {
                cache
                    .set_json(key, &payload, state.cache_ttl_active_projects_count_secs)
                    .await;
            }
            (StatusCode::OK, Json(payload)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /stats`
///
/// Returns pre-calculated global protocol statistics.
pub async fn get_stats(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match db::get_global_stats(&state.pool).await {
        Ok(stats) => {
            let total_terminal = stats.completed_projects + stats.failed_projects;
            let success_rate = if total_terminal > 0 {
                stats.completed_projects as f64 / total_terminal as f64
            } else {
                0.0
            };

            let payload = StatsResponse {
                total_projects: stats.total_projects,
                total_tvl: stats.total_tvl,
                total_donors: stats.total_donors,
                completed_projects: stats.completed_projects,
                failed_projects: stats.failed_projects,
                success_rate,
            };
            (StatusCode::OK, Json(payload)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `POST /webhooks`
///
/// Register a webhook endpoint and subscribed event types.
pub async fn register_webhook(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<RegisterWebhookRequest>,
) -> impl IntoResponse {
    if !(payload.url.starts_with("http://") || payload.url.starts_with("https://")) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!(ErrorResponse {
                error: "Webhook URL must start with http:// or https://".to_string()
            })),
        )
            .into_response();
    }
    if payload.secret.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!(ErrorResponse {
                error: "Webhook secret cannot be empty".to_string()
            })),
        )
            .into_response();
    }

    let registration = db::NewWebhookRegistration {
        url: payload.url,
        secret: payload.secret,
        event_types: payload.event_types,
    };

    match db::create_webhook(&state.pool, &registration).await {
        Ok(webhook) => (StatusCode::CREATED, Json(webhook)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /webhooks`
///
/// List all registered webhooks and subscriptions.
pub async fn list_webhooks(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    match db::list_webhooks(&state.pool).await {
        Ok(webhooks) => (StatusCode::OK, Json(webhooks)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!(ErrorResponse {
                error: e.to_string()
            })),
        )
            .into_response(),
    }
}
