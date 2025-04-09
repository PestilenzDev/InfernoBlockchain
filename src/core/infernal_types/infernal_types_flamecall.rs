// src/core/infernal_types/infernal_types_flamecall.rs

use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};
use log::{debug, info};
use crate::core::error::InfernoError;
use schnorrkel::{PublicKey, Signature};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Flamecall {
    pub id: u64,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub gas_limit: u64,
    pub timestamp: u64,
    pub core: FlamecallCore,
    pub success_rate: f32,
    pub mev_value: f32,
    pub nonce: u64,
    pub zk_proof: Option<(Vec<u8>, Signature)>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FlamecallCore {
    pub id: u64,
    pub data: Vec<u8>,
}

impl Flamecall {
    #[inline]
    pub fn new(
        id: u64,
        sender: &str,
        recipient: &str,
        amount: u64,
        gas_limit: u64,
        nonce: u64,
        data: Option<Vec<u8>>,
        zk_proof: Option<(Vec<u8>, PublicKey)>,
    ) -> Result<Self, InfernoError> {
        if sender.len() > 64 || sender.is_empty() {
            return Err(InfernoError::ParseError("Sender address must be 1-64 chars".into()));
        }
        if recipient.len() > 64 || recipient.is_empty() {
            return Err(InfernoError::ParseError("Recipient address must be 1-64 chars".into()));
        }
        if amount == 0 {
            return Err(InfernoError::Network("Amount must be greater than 0".into()));
        }
        if gas_limit < 21_000 {
            return Err(InfernoError::Network("Gas limit must be at least 21,000".into()));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| InfernoError::Network(e.to_string()))?
            .as_secs();
        let core = FlamecallCore {
            id,
            data: data.unwrap_or_default(),
        };

        let zk_proof_signature = zk_proof.map(|(proof, pub_key)| {
            let hash = {
                let mut hasher = Sha256::new();
                hasher.update(&proof);
                hasher.update(id.to_be_bytes());
                hex::encode(hasher.finalize())
            };
            pub_key
                .verify_simple(b"zk_proof", hash.as_bytes(), &Signature::from_bytes(&proof).unwrap())
                .map_err(|e| InfernoError::Network(e.to_string()))?;
            Ok::<_, InfernoError>((proof, Signature::from_bytes(&proof).unwrap()))
        }).transpose()?;

        let tx = Self {
            id,
            sender: sender.to_string(),
            recipient: recipient.to_string(),
            amount,
            gas_limit,
            timestamp,
            core,
            success_rate: 1.0,
            mev_value: 0.0,
            nonce,
            zk_proof: zk_proof_signature,
        };
        info!("Created Flamecall {}: sender={}, recipient={}, nonce={}", id, sender, recipient, nonce);
        Ok(tx)
    }

    #[inline]
    pub fn update_success_rate(&mut self, rate: f32) -> Result<(), InfernoError> {
        if rate.is_nan() || rate.is_infinite() {
            return Err(InfernoError::ParseError("Success rate must be a valid number".into()));
        }
        self.success_rate = rate.clamp(0.0, 1.0);
        Ok(())
    }

    #[inline]
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.core.id.to_be_bytes());
        hasher.update(&self.core.data);
        hasher.update(self.nonce.to_be_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    #[inline]
    pub fn verify_zk_proof(&self, public_key: &PublicKey) -> Result<bool, InfernoError> {
        if let Some((proof, signature)) = &self.zk_proof {
            let hash = {
                let mut hasher = Sha256::new();
                hasher.update(proof);
                hasher.update(self.id.to_be_bytes());
                hex::encode(hasher.finalize())
            };
            public_key
                .verify_simple(b"zk_proof", hash.as_bytes(), signature)
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
    fn test_flamecall_nonce() {
        let tx1 = Flamecall::new(8, "Alice", "Bob", 100, 100_000, 1, None, None).unwrap();
        let tx2 = Flamecall::new(8, "Alice", "Bob", 100, 100_000, 2, None, None).unwrap();
        assert_ne!(tx1.hash(), tx2.hash());
    }

    #[test]
    fn test_flamecall_zk_proof() {
        let keys = InfernalKeys::new().unwrap();
        let proof = vec![1, 2, 3];
        let tx = Flamecall::new(
            9,
            "Charlie",
            "Dave",
            200,
            200_000,
            1,
            None,
            Some((proof.clone(), keys.public_key())),
        ).unwrap();
        assert_eq!(tx.zk_proof.as_ref().map(|(p, _)| p.clone()), Some(proof));
        assert!(tx.verify_zk_proof(&keys.public_key()).unwrap());
    }
}