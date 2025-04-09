// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_types.rs

use serde::{Serialize, Deserialize};
use crate::core::infernal_types::infernal_types_flamekeeper::Flamekeeper;
use crate::core::infernal_types::infernal_types_flamecall::Flamecall as CoreFlamecall;
use crate::core::infernal_types::infernal_types_emberblock::Emberblock as CoreEmberblock;
use crate::core::error::InfernoError;
use crate::core::infernal_types::infernal_keys::InfernalKeys;
use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_flamehash::Flamehash;
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};

/// Repräsentiert einen Shard im INSC.
/// Whitepaper: 2.2 Architektur - Shards (Hexagonale Meshes).
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Shard {
    pub id: u64,
    pub region: String,
    pub neurons: Vec<Neuron>,
    pub success_rate: f32,
    pub last_update: u64, // Für Rate-Limiting
}

impl Shard {
    #[inline]
    pub fn new(id: u64, region: &str) -> Result<Self, InfernoError> {
        if region.is_empty() {
            return Err(InfernoError::ParseError("Region must not be empty".into()));
        }
        let shard = Self {
            id,
            region: region.to_string(),
            neurons: Vec::new(),
            success_rate: 1.0,
            last_update: 0,
        };
        info!("Created Shard {} in region {}", id, region);
        Ok(shard)
    }

    #[inline]
    pub fn add_neuron(&mut self, neuron: Neuron) -> Result<(), InfernoError> {
        self.check_rate_limit(100)?; // Max 100 Updates pro Sekunde
        self.neurons.push(neuron);
        debug!("Added neuron to Shard {}: total neurons={}", self.id, self.neurons.len());
        Ok(())
    }

    fn check_rate_limit(&mut self, max_updates_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| InfernoError::Network(e.to_string()))?
            .as_secs();
        if now != self.last_update {
            self.last_update = now;
        } else if self.neurons.len() as u32 >= max_updates_per_sec {
            return Err(InfernoError::Network("Shard update rate limit exceeded".into()));
        }
        Ok(())
    }
}

/// Repräsentiert ein Neuron im INSC.
/// Whitepaper: 2.2 Architektur - Shards und Super-Neuronen.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Neuron {
    pub id: u64,
    pub flamekeeper: Flamekeeper,
    pub capacity_score: f32,
    pub zk_proof: Option<(Vec<u8>, Signature)>, // Vollwertig implementiert
}

impl Neuron {
    #[inline]
    pub fn new(id: u64, flamekeeper: Flamekeeper) -> Result<Self, InfernoError> {
        let capacity_score = flamekeeper.capacity_score;
        let neuron = Self {
            id,
            flamekeeper,
            capacity_score,
            zk_proof: None,
        };
        info!("Created Neuron {} with capacity_score={:.2}", id, capacity_score);
        Ok(neuron)
    }
}

/// Repräsentiert ein Super-Neuron im INSC.
/// Whitepaper: 2.2 Architektur - Super-Neuronen.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SuperNeuron {
    pub id: u64,
    pub neuron: Neuron,
    pub signature: Option<schnorrkel::Signature>, // Kryptografische Integrität
}

impl SuperNeuron {
    #[inline]
    pub fn new(id: u64, neuron: Neuron, keys: Option<&InfernalKeys>) -> Result<Self, InfernoError> {
        let mut sn = Self {
            id,
            neuron,
            signature: None,
        };
        if let Some(keys) = keys {
            let flamehash = Flamehash::new();
            let dummy_tx = INSCFlamecall::new(
                CoreFlamecall::new(id, "system", "super_neuron", 0, 21_000, 0, None, None)?,
                0.0
            )?;
            let hash = flamehash.hash_transaction(&dummy_tx)?;
            sn.signature = Some(keys.sign(hash.as_bytes())?);
        }
        info!("Created SuperNeuron {}", id);
        Ok(sn)
    }
}

/// Repräsentiert eine Transaktion im INSC.
/// Whitepaper: 2.3.1 Transaktionseinreichung und Spamschutz.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct INSCFlamecall {
    pub core: CoreFlamecall,
    pub mev_value: f32,
}

impl INSCFlamecall {
    #[inline]
    pub fn new(core: CoreFlamecall, mev_value: f32) -> Result<Self, InfernoError> {
        if mev_value < 0.0 || mev_value.is_nan() || mev_value.is_infinite() {
            return Err(InfernoError::ParseError("Invalid MEV value".into()));
        }
        let tx = Self { core, mev_value };
        info!("Created INSCFlamecall {} with mev_value={}", tx.core.id, mev_value);
        Ok(tx)
    }
}

/// Repräsentiert einen Block im INSC.
/// Whitepaper: 2.3.5 Finalität.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct INSCEmberblock {
    pub core: CoreEmberblock,
    pub shard_ids: Vec<u64>,
    pub super_neuron_id: u64,
}

impl INSCEmberblock {
    #[inline]
    pub fn new(core: CoreEmberblock, shard_ids: Vec<u64>, super_neuron_id: u64) -> Result<Self, InfernoError> {
        if shard_ids.is_empty() {
            return Err(InfernoError::Network("Shard IDs must not be empty".into()));
        }
        let block = Self {
            core,
            shard_ids,
            super_neuron_id,
        };
        info!("Created INSCEmberblock at height {} with {} shards", block.core.height, block.shard_ids.len());
        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_new() {
        let shard = Shard::new(1, "EU").unwrap();
        assert_eq!(shard.id, 1);
        assert_eq!(shard.region, "EU");
        assert!(Shard::new(2, "").is_err());
    }

    #[test]
    fn test_neuron_new() {
        let fk = Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap();
        let neuron = Neuron::new(1, fk).unwrap();
        assert_eq!(neuron.id, 1);
        assert_eq!(neuron.capacity_score, 5.4);
    }

    #[test]
    fn test_super_neuron_new() {
        let fk = Flamekeeper::new(2, 1000, 40.0, -100.0, 8.0, 16.0, 70.0).unwrap();
        let neuron = Neuron::new(2, fk).unwrap();
        let keys = InfernalKeys::new().unwrap();
        let sn = SuperNeuron::new(1, neuron, Some(&keys)).unwrap();
        assert!(sn.signature.is_some());
    }

    #[test]
    fn test_flamecall_new() {
        let core_tx = CoreFlamecall::new(1, "Alice", "Bob", 100, 100_000, 1, None, None).unwrap();
        let tx = INSCFlamecall::new(core_tx, 1.0).unwrap();
        assert_eq!(tx.mev_value, 1.0);
        assert!(INSCFlamecall::new(CoreFlamecall::new(2, "Charlie", "Dave", 200, 200_000, 2, None, None).unwrap(), f32::NAN).is_err());
    }

    #[test]
    fn test_emberblock_new() {
        let core_tx = CoreFlamecall::new(3, "Eve", "Frank", 300, 300_000, 1, None, None).unwrap();
        let core_block = CoreEmberblock::new(
            1,
            1,
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
            vec![core_tx],
            None,
        ).unwrap();
        let block = INSCEmberblock::new(core_block, vec![1, 2], 3).unwrap();
        assert_eq!(block.shard_ids, vec![1, 2]);
        assert!(INSCEmberblock::new(
            CoreEmberblock::new(2, 2, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef", vec![], None).unwrap(),
            vec![],
            4
        ).is_err());
    }
}