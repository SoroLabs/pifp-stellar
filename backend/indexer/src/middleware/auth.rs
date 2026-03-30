//! Authentication middleware for the PIFP indexer API.
//!
//! Provides Ed25519 signature verification for profile operations.
//! The client must sign the canonical message `"pifp-profile:{address}"` with
//! the Ed25519 private key corresponding to `address`.

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

/// Errors that can occur during authentication.
#[derive(Debug, PartialEq)]
pub enum AuthError {
    /// The `Authorization` header or token is missing.
    MissingToken,
    /// The provided address is not a valid Stellar public key.
    InvalidAddress,
    /// The signature is not valid base64 or not 64 bytes.
    MalformedSignature,
    /// The signature does not match the expected message.
    InvalidSignature,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Missing authorization token"),
            AuthError::InvalidAddress => write!(f, "Invalid Stellar address"),
            AuthError::MalformedSignature => write!(f, "Malformed signature"),
            AuthError::InvalidSignature => write!(f, "Invalid signature"),
        }
    }
}

/// Verifies an Ed25519 signature over `"pifp-profile:{address}"`.
///
/// Returns `Ok(())` on success, or an [`AuthError`] describing the failure.
pub fn verify_profile_signature(address: &str, signature_b64: &str) -> Result<(), AuthError> {
    if signature_b64.is_empty() {
        return Err(AuthError::MissingToken);
    }

    let strkey = StellarPublicKey::from_string(address).map_err(|_| AuthError::InvalidAddress)?;

    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|_| AuthError::MalformedSignature)?;

    let sig_array: &[u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| AuthError::MalformedSignature)?;

    let sig = Signature::from_bytes(sig_array);

    let vk = VerifyingKey::from_bytes(&strkey.0).map_err(|_| AuthError::InvalidAddress)?;

    let message = format!("pifp-profile:{address}");
    vk.verify(message.as_bytes(), &sig)
        .map_err(|_| AuthError::InvalidSignature)
}
