use axum::{
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use crate::ipfs::{pin_file, IpfsConfig};
use crate::errors::Result;

#[derive(Serialize)]
pub struct UploadResponse {
    pub cid: String,
    pub status: String,
}

pub struct IpfsState {
    pub config: IpfsConfig,
}

pub fn router(state: Arc<IpfsState>) -> Router {
    Router::new()
        .route("/ipfs/upload", post(upload_file))
        .with_state(state)
}

async fn upload_file(
    State(state): State<Arc<IpfsState>>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>> {
    let mut file_data = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| crate::errors::OracleError::Network(format!("Multipart error: {}", e)))? {
        if let Some(name) = field.name() {
            if name == "file" {
                file_data = field.bytes().await.map_err(|e| crate::errors::OracleError::Network(format!("Field bytes error: {}", e)))?.to_vec();
                break;
            }
        }
    }

    if file_data.is_empty() {
        return Err(crate::errors::OracleError::Verification("No file data provided".to_string()));
    }

    let cid = pin_file(file_data, &state.config).await?;

    Ok(Json(UploadResponse {
        cid,
        status: "success".to_string(),
    }))
}
