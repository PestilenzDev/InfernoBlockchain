// src/core/infernal_types/infernal_types_emberblock.rs

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::core::infernal_types::infernal_types_flamecall::Flamecall;
use crate::core::infernal_types::infernal_keys::InfernalKeys;
use crate::core::error::InfernoError;
use schnorrkel::Signature;
use log::{debug, info};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Emberblock {
    pub height: u64,
    pub timestamp: u64,
    pub transactions: Vec<Flamecall>,
    pub shard_id: u64,
    pub previous_hash: String,
    pub approval_rate: f32,
    pub data: Option<Vec<u8>>, // Vollwertig für Hashing
    /// Signatur des Blocks für kryptografische Integrität.
    pub signature: Option<Signature>,
}

impl Emberblock {
    #[inline]
    pub fn new(
        height: u64,
        shard_id: u64,
        previous_hash: &str,
        transactions: Vec<Flamecall>,
        keys: Option<&InfernalKeys>,
    ) -> Result<Self, InfernoError> {
        const MAX_TXS: usize = 10_000;
        if transactions.len() > MAX_TXS {
            return Err(InfernoError::Network(format!(
                "Exceeded max transactions: {} > {}",
                transactions.len(), MAX_TXS
            )));
        }
        if previous_hash.len() != 64 || !previous_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(InfernoError::ParseError("Previous hash must be 64 hex chars".into()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| InfernoError::Network(e.to_string()))?
            .as_secs();
        let mut block = Self {
            height,
            timestamp,
            transactions,
            shard_id,
            previous_hash: previous_hash.to_string(),
            approval_rate: 0.0,
            data: None,
            signature: None,
        };
        
        if let Some(keys) = keys {
            let hash = block.hash();
            block.signature = Some(keys.sign(hash.as_bytes())?);
            block.data = Some(hash.as_bytes().to_vec()); // Vollwertige Hash-Daten
        }
        
        info!("Created Emberblock {}: shard_id={}, tx_count={}", height, shard_id, transactions.len());
        Ok(block)
    }

    #[inline]
    pub fn update_approval(&mut self, rate: f32) -> Result<(), InfernoError> {
        if rate.is_nan() || rate.is_infinite() {
            return Err(InfernoError::ParseError("Approval rate must be a valid number".into()));
        }
        self.approval_rate = rate.clamp(0.0, 1.0);
        Ok(())
    }

    #[inline]
    pub fn calculate_tps(&self, duration_secs: f64) -> Result<f64, InfernoError> {
        if duration_secs <= 0.0 {
            return Err(InfernoError::Network("Duration must be positive".into()));
        }
        Ok(self.transactions.len() as f64 / duration_secs)
    }

    #[inline]
    pub fn hash(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(self.height.to_be_bytes());
        hasher.update(self.timestamp.to_be_bytes());
        hasher.update(self.shard_id.to_be_bytes());
        hasher.update(self.previous_hash.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    pub fn verify_signature(&self, public_key: &schnorrkel::PublicKey) -> Result<bool, InfernoError> {
        if let Some(sig) = &self.signature {
            let hash = self.hash();
            public_key.verify_simple(b"inferno", hash.as_bytes(), sig)
                .map_err(|e| InfernoError::Network(e.to_string()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::infernal_types::infernal_keys::InfernalKeys;

    #[test]
    fn test_emberblock_signature() {
        let keys = InfernalKeys::new().unwrap();
        let txs = vec![Flamecall::new(10, "Alice", "Bob", 100, 100_000, 1, None, None).unwrap()];
        let block = Emberblock::new(5, 4, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef", txs, Some(&keys)).unwrap();
        assert!(block.signature.is_some());
        assert!(block.data.is_some());
        assert!(block.verify_signature(&keys.public_key()).unwrap());
    }
}