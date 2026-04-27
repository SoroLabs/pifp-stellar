use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    #[serde(rename = "user_id")]
    pub _user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct IssueRequest {
    pub user_id: String,
    pub challenge: String,
    pub claims: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub credential: String,
    pub requested_claims: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifiableCredential {
    pub r#type: Vec<String>,
    pub credential_subject: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub vc: VerifiableCredential,
    pub exp: usize,
}

pub async fn request_challenge(
    Json(_payload): Json<ChallengeRequest>,
) -> impl IntoResponse {
    // Simple credible abstraction of a challenge
    Json(serde_json::json!({ "challenge": "pifp-auth-nonce-99" }))
}

pub async fn issue_credential(
    Json(payload): Json<IssueRequest>,
) -> impl IntoResponse {
    // Validate challenge (minimal check)
    if payload.challenge != "pifp-auth-nonce-99" {
        return (StatusCode::BAD_REQUEST, "Invalid challenge").into_response();
    }

    let claims = Claims {
        iss: "did:pifp:oracle".to_string(),
        sub: payload.user_id,
        vc: VerifiableCredential {
            r#type: vec!["VerifiableCredential".to_string()],
            credential_subject: payload.claims,
        },
        exp: 2_000_000_000,
    };

    let key = EncodingKey::from_secret("secret".as_ref());
    match encode(&Header::default(), &claims, &key) {
        Ok(t) => Json(serde_json::json!({ "credential": t })).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn verify_credential(
    Json(payload): Json<VerifyRequest>,
) -> impl IntoResponse {
    let decoding_key = DecodingKey::from_secret("secret".as_ref());
    let token_data = match decode::<Claims>(&payload.credential, &decoding_key, &Validation::default()) {
        Ok(d) => d,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let requested: HashSet<String> = payload.requested_claims.into_iter().collect();
    let mut disclosed = serde_json::Map::new();
    
    if let Some(subject) = token_data.claims.vc.credential_subject.as_object() {
        // 1. Direct Field Disclosure
        for field in &requested {
            if let Some(val) = subject.get(field) {
                disclosed.insert(field.clone(), val.clone());
            }
        }

        // 2. Derived Claim: isAgeOver18 (Simulated ZKP/Predicate)
        if requested.contains("isAgeOver18") {
            if let Some(age) = subject.get("age").and_then(|a| a.as_u64()) {
                disclosed.insert("isAgeOver18".to_string(), serde_json::json!(age >= 18));
            }
        }
    }

    Json(serde_json::json!({
        "valid": true,
        "issuer": token_data.claims.iss,
        "disclosed": disclosed
    })).into_response()
}
