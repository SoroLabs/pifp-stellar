use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf: String,
    pub index: usize,
    pub siblings: Vec<String>,
}

pub struct StateMerkleTree {
    pub leaves: Vec<[u8; 32]>,
    pub layers: Vec<Vec<[u8; 32]>>,
}

impl StateMerkleTree {
    pub fn new(state: &HashMap<String, String>) -> Self {
        let mut sorted_keys: Vec<_> = state.keys().collect();
        sorted_keys.sort();

        let leaves: Vec<[u8; 32]> = sorted_keys
            .into_iter()
            .map(|k| {
                let v = state.get(k).unwrap();
                let mut hasher = Sha256::new();
                hasher.update(k.as_bytes());
                hasher.update(b":");
                hasher.update(v.as_bytes());
                hasher.finalize().into()
            })
            .collect();

        let mut layers = vec![leaves.clone()];
        let mut current_layer = leaves.clone();

        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();
            for i in (0..current_layer.len()).step_by(2) {
                let left = current_layer[i];
                let right = if i + 1 < current_layer.len() {
                    current_layer[i + 1]
                } else {
                    left // odd number of leaves, duplicate last one
                };

                let mut hasher = Sha256::new();
                if left <= right {
                    hasher.update(left);
                    hasher.update(right);
                } else {
                    hasher.update(right);
                    hasher.update(left);
                }
                next_layer.push(hasher.finalize().into());
            }
            layers.push(next_layer.clone());
            current_layer = next_layer;
        }

        Self { leaves, layers }
    }

    pub fn root(&self) -> String {
        if self.layers.is_empty() || self.layers.last().unwrap().is_empty() {
            return hex::encode([0u8; 32]);
        }
        hex::encode(self.layers.last().unwrap()[0])
    }

    pub fn get_proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaves.len() {
            return None;
        }

        let mut siblings = Vec::new();
        let mut current_idx = index;

        for layer in &self.layers {
            if layer.len() <= 1 {
                break;
            }

            let sibling_idx = if current_idx % 2 == 0 {
                if current_idx + 1 < layer.len() {
                    current_idx + 1
                } else {
                    current_idx
                }
            } else {
                current_idx - 1
            };

            siblings.push(hex::encode(layer[sibling_idx]));
            current_idx /= 2;
        }

        Some(MerkleProof {
            leaf: hex::encode(self.leaves[index]),
            index,
            siblings,
        })
    }
}

pub fn verify_proof(root: &str, leaf: &str, proof: &MerkleProof) -> bool {
    let mut current_hash = hex::decode(leaf).unwrap_or_default();
    
    for sibling_hex in &proof.siblings {
        let sibling = hex::decode(sibling_hex).unwrap_or_default();
        let mut hasher = Sha256::new();
        
        let (left, right) = if current_hash.as_slice() <= sibling.as_slice() {
            (current_hash, sibling)
        } else {
            (sibling, current_hash)
        };
        
        hasher.update(left);
        hasher.update(right);
        current_hash = hasher.finalize().into();
    }
    
    hex::encode(current_hash) == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree() {
        let mut state = HashMap::new();
        state.insert("total_donations".to_string(), "1000".to_string());
        state.insert("donor_count".to_string(), "50".to_string());
        state.insert("project_id".to_string(), "1".to_string());

        let tree = StateMerkleTree::new(&state);
        let root = tree.root();
        println!("Root: {}", root);

        for i in 0..state.len() {
            let proof = tree.get_proof(i).unwrap();
            assert!(verify_proof(&root, &proof.leaf, &proof));
        }
    }
}
