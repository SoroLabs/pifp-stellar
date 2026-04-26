use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedIntent {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub nonce: u64,
    pub pubkey_hex: String,
    pub signature_hex: String,
}

impl SignedIntent {
    pub fn signing_payload(&self) -> Vec<u8> {
        format!("{}:{}:{}:{}", self.from, self.to, self.amount, self.nonce).into_bytes()
    }

    pub fn verify_signature(&self) -> Result<(), String> {
        let pubkey_bytes = hex::decode(&self.pubkey_hex).map_err(|e| e.to_string())?;
        let signature_bytes = hex::decode(&self.signature_hex).map_err(|e| e.to_string())?;
        let key = VerifyingKey::from_bytes(
            &pubkey_bytes
                .try_into()
                .map_err(|_| "invalid ed25519 pubkey length")?,
        )
        .map_err(|e| e.to_string())?;
        let signature = Signature::from_bytes(
            &signature_bytes
                .try_into()
                .map_err(|_| "invalid ed25519 signature length")?,
        );

        key.verify(&self.signing_payload(), &signature)
            .map_err(|e| e.to_string())
    }
}

pub fn account_key(account: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(account.as_bytes());
    let out = hasher.finalize();
    u64::from_be_bytes(out[..8].try_into().expect("slice length is 8"))
}

