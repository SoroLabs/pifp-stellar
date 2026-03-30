//! Unit tests for the authentication middleware (closes #155).
//!
//! Covers 100% branch coverage of `verify_profile_signature`:
//!   - Valid token (correct key + signature)
//!   - Expired / wrong-message token (signature over different content)
//!   - Malformed token (bad base64, wrong length, invalid address)
//!   - Missing token (empty signature string)

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use stellar_strkey::ed25519::PublicKey as StellarPublicKey;

use crate::middleware::auth::{verify_profile_signature, AuthError};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Generate a fresh keypair and return (stellar_address, signing_key).
fn make_keypair() -> (String, SigningKey) {
    let sk = SigningKey::generate(&mut OsRng);
    let address = StellarPublicKey(sk.verifying_key().to_bytes()).to_string();
    (address, sk)
}

/// Sign the canonical profile message and return base64-encoded signature.
fn sign_profile(address: &str, sk: &SigningKey) -> String {
    let msg = format!("pifp-profile:{address}");
    let sig = sk.sign(msg.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(sig.to_bytes())
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[test]
fn valid_token_grants_access() {
    let (address, sk) = make_keypair();
    let sig = sign_profile(&address, &sk);
    assert_eq!(verify_profile_signature(&address, &sig), Ok(()));
}

#[test]
fn wrong_message_signature_is_rejected() {
    // Simulates an "expired" or replayed token signed over different content.
    let (address, sk) = make_keypair();
    let stale_sig = {
        let msg = "pifp-profile:some-other-address";
        let sig = sk.sign(msg.as_bytes());
        base64::engine::general_purpose::STANDARD.encode(sig.to_bytes())
    };
    assert_eq!(
        verify_profile_signature(&address, &stale_sig),
        Err(AuthError::InvalidSignature)
    );
}

#[test]
fn signature_from_different_key_is_rejected() {
    let (address, _) = make_keypair();
    let (_, other_sk) = make_keypair();
    // Sign with a key that doesn't match `address`.
    let sig = sign_profile(&address, &other_sk);
    assert_eq!(
        verify_profile_signature(&address, &sig),
        Err(AuthError::InvalidSignature)
    );
}

#[test]
fn invalid_base64_signature_is_rejected() {
    let (address, _) = make_keypair();
    assert_eq!(
        verify_profile_signature(&address, "not!!valid==base64$$"),
        Err(AuthError::MalformedSignature)
    );
}

#[test]
fn wrong_length_signature_is_rejected() {
    let (address, _) = make_keypair();
    // Valid base64 but only 32 bytes — not a valid Ed25519 signature (needs 64).
    let short = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
    assert_eq!(
        verify_profile_signature(&address, &short),
        Err(AuthError::MalformedSignature)
    );
}

#[test]
fn invalid_stellar_address_is_rejected() {
    let (_, sk) = make_keypair();
    let fake_address = "GBADADDRESS";
    let sig = sign_profile(fake_address, &sk);
    assert_eq!(
        verify_profile_signature(fake_address, &sig),
        Err(AuthError::InvalidAddress)
    );
}

#[test]
fn missing_token_is_rejected() {
    let (address, _) = make_keypair();
    assert_eq!(
        verify_profile_signature(&address, ""),
        Err(AuthError::MissingToken)
    );
}
