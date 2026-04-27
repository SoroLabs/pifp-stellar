use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use crate::ipfs::{pin_file, IpfsConfig};

#[derive(Serialize)]
pub struct UploadResponse {
    pub cid: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct UploadErrorResponse {
    pub error: String,
}

pub struct IpfsState {
    pub config: IpfsConfig,
}

pub fn router() -> Router<Arc<crate::health::ServerState>> {
    Router::new()
        .route("/ipfs/upload", post(upload_file))
}

async fn upload_file(
    State(state): State<Arc<crate::health::ServerState>>,
    mut multipart: Multipart,
) -> std::result::Result<Json<UploadResponse>, (StatusCode, Json<UploadErrorResponse>)> {
    let mut file_data = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(UploadErrorResponse {
                    error: format!("Multipart error: {e}"),
                }),
            )
        })?
    {
        if let Some(name) = field.name() {
            if name == "file" {
                file_data = field
                    .bytes()
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(UploadErrorResponse {
                                error: format!("Field bytes error: {e}"),
                            }),
                        )
                    })?
                    .to_vec();
                break;
            }
        }
    }

    if file_data.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(UploadErrorResponse {
                error: "No file data provided".to_string(),
            }),
        ));
    }

    let cid = pin_file(file_data, &state.config).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(UploadErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(UploadResponse {
        cid,
        status: "success".to_string(),
    }))
}
