// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_storage.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::{Shard, Neuron, INSCFlamecall};
use crate::core::error::InfernoError;
use log::{debug, info};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use zstd::stream::encode_all;

#[derive(Debug)]
pub struct ConsensusStorage {
    pub shards: HashMap<u64, Shard>, // Öffentlich gemacht
    last_update: u64,
}

impl ConsensusStorage {
    #[inline]
    pub fn new() -> Result<Self, InfernoError> {
        let storage = Self {
            shards: HashMap::new(),
            last_update: 0,
        };
        info!("Initialized ConsensusStorage");
        Ok(storage)
    }

    #[inline]
    pub fn store_shard(&mut self, shard: Shard) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        let compressed = encode_all(&shard.neurons[..], 3)?;
        self.shards.insert(shard.id, shard);
        debug!("Stored shard {} with {} neurons, compressed size={} bytes", self.shards[&self.shards.keys().next().unwrap()].id, self.shards[&self.shards.keys().next().unwrap()].neurons.len(), compressed.len());
        Ok(())
    }

    #[inline]
    pub fn retrieve_shard(&self, shard_id: u64) -> Result<&Shard, InfernoError> {
        self.shards.get(&shard_id).ok_or_else(|| InfernoError::Network(format!("Shard {} not found", shard_id)))
    }

    #[inline]
    pub fn update_neuron(&mut self, shard_id: u64, neuron: Neuron) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        if let Some(shard) = self.shards.get_mut(&shard_id) {
            if let Some(index) = shard.neurons.iter().position(|n| n.id == neuron.id) {
                shard.neurons[index] = neuron;
                debug!("Updated neuron {} in shard {}", shard.neurons[index].id, shard_id);
                Ok(())
            } else {
                Err(InfernoError::Network(format!("Neuron {} not found in shard {}", neuron.id, shard_id)))
            }
        } else {
            Err(InfernoError::Network(format!("Shard {} not found", shard_id)))
        }
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_update {
            self.last_update = now;
        } else if max_per_sec == 0 {
            return Err(InfernoError::Network("Storage rate limit exceeded".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_operations() {
        let mut storage = ConsensusStorage::new().unwrap();
        let mut shard = Shard::new(1, "EU").unwrap();
        let fk = Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap();
        shard.add_neuron(Neuron::new(1, fk.clone()).unwrap()).unwrap();
        
        assert!(storage.store_shard(shard).is_ok());
        assert_eq!(storage.shards.len(), 1);

        let retrieved = storage.retrieve_shard(1).unwrap();
        assert_eq!(retrieved.id, 1);
        assert_eq!(retrieved.neurons.len(), 1);

        let mut updated_neuron = Neuron::new(1, fk).unwrap();
        updated_neuron.flamekeeper.activity = 0.9;
        assert!(storage.update_neuron(1, updated_neuron).is_ok());
        assert_eq!(storage.shards[&1].neurons[0].flamekeeper.activity, 0.9);
    }

    #[test]
    fn test_storage_public_shards() {
        let mut storage = ConsensusStorage::new().unwrap();
        let shard = Shard::new(1, "EU").unwrap();
        storage.store_shard(shard).unwrap();
        assert_eq!(storage.shards.len(), 1); // Öffentlicher Zugriff
    }
}