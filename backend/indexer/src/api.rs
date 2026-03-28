//! Axum REST API handlers.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

use crate::db;
use crate::events::EventRecord;
use crate::profiles::{self, ProfileUpdate};

#[derive(Clone)]
pub struct ApiState {
    pub pool: SqlitePool,
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

fn verify_profile_signature(address: &str, signature_b64: &str) -> bool {
    let Ok(strkey) = StellarPublicKey::from_string(address) else {
        return false;
    };
    let Ok(sig_bytes) = base64::engine::general_purpose::STANDARD.decode(signature_b64) else {
        return false;
    };
    let Ok(sig_array): Result<&[u8; 64], _> = sig_bytes.as_slice().try_into() else {
        return false;
    };
    let sig = Signature::from_bytes(sig_array);
    let Ok(vk) = VerifyingKey::from_bytes(&strkey.0) else {
        return false;
    };
    let message = format!("pifp-profile:{address}");
    use ed25519_dalek::Verifier;
    vk.verify(message.as_bytes(), &sig).is_ok()
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

/// `GET /projects/:id/events`
///
/// Returns all indexed events for the given project identifier.
pub async fn get_project_events(
    State(state): State<Arc<ApiState>>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match db::get_events_for_project(&state.pool, &project_id).await {
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

    if !verify_profile_signature(&address, &payload.signature) {
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

    if !verify_profile_signature(&address, &payload.signature) {
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
