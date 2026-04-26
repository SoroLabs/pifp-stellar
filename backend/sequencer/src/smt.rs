use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::types::account_key;

pub type Hash = [u8; 32];
const DEPTH: usize = 32;

#[derive(Debug, Clone)]
pub struct BalanceWitness {
    pub account: String,
    pub old_balance: u64,
    pub new_balance: u64,
    pub siblings: Vec<Hash>,
}

#[derive(Debug, Clone)]
pub struct SparseMerkleTree {
    balances: HashMap<u64, u64>,
    nodes: HashMap<(usize, u64), Hash>,
    defaults: Vec<Hash>,
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        let mut defaults = Vec::with_capacity(DEPTH + 1);
        defaults.push(hash_leaf(0));
        for level in 1..=DEPTH {
            let prev = defaults[level - 1];
            defaults.push(hash_pair(prev, prev));
        }

        Self {
            balances: HashMap::new(),
            nodes: HashMap::new(),
            defaults,
        }
    }

    pub fn root(&self) -> Hash {
        *self
            .nodes
            .get(&(DEPTH, 0))
            .unwrap_or(&self.defaults[DEPTH])
    }

    pub fn balance_of(&self, account: &str) -> u64 {
        let key = account_key(account);
        self.balances.get(&key).copied().unwrap_or(0)
    }

    pub fn set_balance(&mut self, account: &str, new_balance: u64) -> BalanceWitness {
        let key = account_key(account);
        let old_balance = self.balances.get(&key).copied().unwrap_or(0);
        self.balances.insert(key, new_balance);

        let mut siblings = Vec::with_capacity(DEPTH);
        let mut node_index = key;
        let mut current_hash = hash_leaf(new_balance);
        self.nodes.insert((0, node_index), current_hash);

        for level in 0..DEPTH {
            let sibling_index = node_index ^ 1;
            let sibling_hash = *self
                .nodes
                .get(&(level, sibling_index))
                .unwrap_or(&self.defaults[level]);
            siblings.push(sibling_hash);

            let parent_index = node_index >> 1;
            let (left, right) = if node_index & 1 == 0 {
                (current_hash, sibling_hash)
            } else {
                (sibling_hash, current_hash)
            };
            current_hash = hash_pair(left, right);
            self.nodes.insert((level + 1, parent_index), current_hash);
            node_index = parent_index;
        }

        BalanceWitness {
            account: account.to_string(),
            old_balance,
            new_balance,
            siblings,
        }
    }
}

fn hash_leaf(value: u64) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([0u8]);
    hasher.update(value.to_be_bytes());
    hasher.finalize().into()
}

fn hash_pair(left: Hash, right: Hash) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update([1u8]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::SparseMerkleTree;

    #[test]
    fn root_changes_after_update() {
        let mut smt = SparseMerkleTree::new();
        let root0 = smt.root();
        smt.set_balance("alice", 10);
        let root1 = smt.root();
        assert_ne!(root0, root1);
    }
}

