// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_flamehash.rs

use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::core::error::InfernoError;
use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::{INSCFlamecall, INSCEmberblock};
use crate::core::infernal_types::infernal_keys::InfernalKeys;

#[derive(Clone)]
pub struct Flamehash {
    difficulty: u32,
}

impl Flamehash {
    pub fn new() -> Self {
        Flamehash { difficulty: 4 }
    }

    pub fn hash_transaction(&self, tx: &INSCFlamecall) -> Result<String, InfernoError> {
        let mut hasher = Sha256::new();
        hasher.update(tx.core.id.to_be_bytes());
        hasher.update(tx.core.sender.as_bytes());
        hasher.update(tx.core.recipient.as_bytes());
        hasher.update(tx.core.amount.to_be_bytes());
        hasher.update(tx.core.gas_limit.to_be_bytes());
        hasher.update(tx.core.timestamp.to_be_bytes());
        hasher.update(&tx.core.data);
        hasher.update(tx.core.nonce.to_be_bytes());
        if let Some((proof, _)) = &tx.core.zk_proof {
            hasher.update(proof);
        }
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    pub fn hash_block(&self, block: &INSCEmberblock) -> Result<String, InfernoError> {
        let mut hasher = Sha256::new();
        hasher.update(block.core.height.to_be_bytes());
        hasher.update(block.core.shard_id.to_be_bytes());
        hasher.update(block.core.timestamp.to_be_bytes());
        hasher.update(block.core.previous_hash.as_bytes());
        for tx in &block.core.transactions {
            let tx_hash = self.hash_transaction(&INSCFlamecall::new(tx.clone(), tx.mev_value)?)?;
            hasher.update(tx_hash.as_bytes());
        }
        if let Some(data) = &block.core.data {
            hasher.update(data);
        }
        if let Some(signature) = &block.core.signature {
            hasher.update(signature.to_bytes());
        }
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    pub fn validate(&self, hash: &str) -> bool {
        hash.starts_with(&"0".repeat(self.difficulty as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::infernal_types::infernal_types_flamecall::Flamecall;
    use crate::core::infernal_types::infernal_types_emberblock::Emberblock;

    #[test]
    fn test_hash_transaction() {
        let flamehash = Flamehash::new();
        let core_tx = Flamecall::new(
            1,
            "Alice",
            "Bob",
            100,
            100_000,
            0,
            None,
            None,
        ).unwrap();
        let tx = INSCFlamecall::new(core_tx, 1.0).unwrap();
        let hash = flamehash.hash_transaction(&tx).unwrap();
        assert_eq!(hash.len(), 64);
        assert!(flamehash.validate(&hash));
    }

    #[test]
    fn test_hash_block() {
        let flamehash = Flamehash::new();
        let keys = InfernalKeys::new().unwrap();
        let core_tx = Flamecall::new(
            1,
            "Alice",
            "Bob",
            100,
            100_000,
            0,
            None,
            None,
        ).unwrap();
        let core_block = INSCEmberblock::new(
            Emberblock::new(
                1,
                1,
                "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
                vec![core_tx],
                Some(&keys),
            ).unwrap(),
            vec![1, 2, 3],
            4,
        ).unwrap();
        let hash = flamehash.hash_block(&core_block).unwrap();
        assert_eq!(hash.len(), 64);
        assert!(flamehash.validate(&hash));
    }
}