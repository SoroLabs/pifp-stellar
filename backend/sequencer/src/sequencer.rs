use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::prover::{BatchSubmission, ProverCoordinator};
use crate::smt::{BalanceWitness, Hash, SparseMerkleTree};
use crate::types::SignedIntent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencedTransition {
    pub intent: SignedIntent,
    pub from_before: u64,
    pub from_after: u64,
    pub to_before: u64,
    pub to_after: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessBatch {
    pub batch_id: u64,
    pub pre_root: String,
    pub post_root: String,
    pub transitions: Vec<SequencedTransition>,
    pub balance_witnesses: Vec<BalanceWitnessRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceWitnessRecord {
    pub account: String,
    pub old_balance: u64,
    pub new_balance: u64,
    pub siblings_hex: Vec<String>,
}

impl From<BalanceWitness> for BalanceWitnessRecord {
    fn from(value: BalanceWitness) -> Self {
        Self {
            account: value.account,
            old_balance: value.old_balance,
            new_balance: value.new_balance,
            siblings_hex: value.siblings.into_iter().map(hex::encode).collect(),
        }
    }
}

#[derive(Debug)]
pub struct Sequencer<C: ProverCoordinator> {
    smt: SparseMerkleTree,
    expected_nonce: HashMap<String, u64>,
    batch_id: u64,
    prover: C,
}

impl<C: ProverCoordinator> Sequencer<C> {
    pub fn new(prover: C) -> Self {
        Self {
            smt: SparseMerkleTree::new(),
            expected_nonce: HashMap::new(),
            batch_id: 1,
            prover,
        }
    }

    pub fn balance_of(&self, account: &str) -> u64 {
        self.smt.balance_of(account)
    }

    pub fn credit(&mut self, account: &str, amount: u64) {
        let cur = self.smt.balance_of(account);
        self.smt.set_balance(account, cur.saturating_add(amount));
    }

    pub fn process_batch(
        &mut self,
        intents: Vec<SignedIntent>,
    ) -> Result<(WitnessBatch, BatchSubmission), String> {
        if intents.is_empty() {
            return Err("batch must include at least one intent".to_string());
        }

        let pre_root = self.smt.root();
        let mut transitions = Vec::with_capacity(intents.len());
        let mut balance_witnesses = Vec::with_capacity(intents.len() * 2);

        for intent in intents {
            intent.verify_signature()?;
            self.verify_nonce(&intent)?;

            let from_balance = self.smt.balance_of(&intent.from);
            let to_balance = self.smt.balance_of(&intent.to);
            if from_balance < intent.amount {
                return Err(format!("insufficient balance for {}", intent.from));
            }

            let from_after = from_balance - intent.amount;
            let to_after = to_balance.saturating_add(intent.amount);
            let from_witness = self.smt.set_balance(&intent.from, from_after);
            let to_witness = self.smt.set_balance(&intent.to, to_after);
            balance_witnesses.push(from_witness.into());
            balance_witnesses.push(to_witness.into());

            transitions.push(SequencedTransition {
                intent: intent.clone(),
                from_before: from_balance,
                from_after,
                to_before: to_balance,
                to_after,
            });
            self.expected_nonce
                .insert(intent.from.clone(), intent.nonce.saturating_add(1));
        }

        let witness = WitnessBatch {
            batch_id: self.batch_id,
            pre_root: encode_hash(pre_root),
            post_root: encode_hash(self.smt.root()),
            transitions,
            balance_witnesses,
        };
        self.batch_id = self.batch_id.saturating_add(1);

        let submission = self.prover.prove_and_submit(&witness)?;
        Ok((witness, submission))
    }

    fn verify_nonce(&self, intent: &SignedIntent) -> Result<(), String> {
        let expected = self.expected_nonce.get(&intent.from).copied().unwrap_or(0);
        if intent.nonce != expected {
            return Err(format!(
                "nonce mismatch for {}: expected {}, got {}",
                intent.from, expected, intent.nonce
            ));
        }
        Ok(())
    }
}

fn encode_hash(hash: Hash) -> String {
    format!("0x{}", hex::encode(hash))
}

