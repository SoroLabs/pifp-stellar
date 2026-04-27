pub mod prover;
pub mod sequencer;
pub mod smt;
pub mod types;

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    use crate::{
        prover::ExternalProverClient,
        sequencer::Sequencer,
        types::SignedIntent,
    };

    fn signed_intent(
        from: &str,
        to: &str,
        amount: u64,
        nonce: u64,
        signing_key: &SigningKey,
    ) -> SignedIntent {
        let mut intent = SignedIntent {
            from: from.to_string(),
            to: to.to_string(),
            amount,
            nonce,
            pubkey_hex: hex::encode(signing_key.verifying_key().to_bytes()),
            signature_hex: String::new(),
        };

        let signature = signing_key.sign(&intent.signing_payload());
        intent.signature_hex = hex::encode(signature.to_bytes());
        intent
    }

    #[test]
    fn sequencer_processes_signed_transfer_batch() {
        let mut csprng = OsRng;
        let alice = SigningKey::generate(&mut csprng);
        let bob = SigningKey::generate(&mut csprng);
        let alice_id = hex::encode(alice.verifying_key().to_bytes());
        let bob_id = hex::encode(bob.verifying_key().to_bytes());

        let prover = ExternalProverClient::new("https://gpu-provers.local", "https://rpc.local");
        let mut sequencer = Sequencer::new(prover);
        sequencer.credit(&alice_id, 100);

        let batch = vec![signed_intent(&alice_id, &bob_id, 40, 0, &alice)];
        let (witness, submission) = sequencer
            .process_batch(batch)
            .expect("batch should process");

        assert_eq!(witness.transitions.len(), 1);
        assert_eq!(sequencer.balance_of(&alice_id), 60);
        assert_eq!(sequencer.balance_of(&bob_id), 40);
        assert!(submission.l1_tx_hash.starts_with("0x"));
    }
}

