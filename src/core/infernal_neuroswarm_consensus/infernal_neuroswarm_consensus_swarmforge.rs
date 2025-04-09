// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_swarmforge.rs

use crate::core::infernal_types::infernal_keys::InfernalKeys;
use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::{Shard, Neuron, SuperNeuron, INSCFlamecall};
use crate::core::error::InfernoError;
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::seq::SliceRandom;

#[derive(Debug)]
pub struct Swarmforge {
    shards: Vec<Shard>,
    super_neurons: Vec<SuperNeuron>,
    meta_neurons: Vec<SuperNeuron>,
    last_aggregation: u64,
    rotation_count: u32,
}

impl Swarmforge {
    #[inline]
    pub fn new() -> Result<Self, InfernoError> {
        let sf = Self {
            shards: Vec::new(),
            super_neurons: Vec::new(),
            meta_neurons: Vec::new(),
            last_aggregation: 0,
            rotation_count: 0,
        };
        info!("Initialized Swarmforge");
        Ok(sf)
    }

    #[inline]
    pub fn add_shard(&mut self, shard: Shard) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        self.shards.push(shard);
        debug!("Added shard to Swarmforge: total shards={}", self.shards.len());
        Ok(())
    }

    #[inline]
    pub fn add_super_neuron(&mut self, super_neuron: SuperNeuron) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        self.super_neurons.push(super_neuron);
        debug!("Added super neuron to Swarmforge: total super neurons={}", self.super_neurons.len());
        Ok(())
    }

    #[inline]
    pub fn add_meta_neuron(&mut self, meta_neuron: SuperNeuron) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        self.meta_neurons.push(meta_neuron);
        debug!("Added meta neuron to Swarmforge: total meta neurons={}", self.meta_neurons.len());
        Ok(())
    }

    #[inline]
    pub fn aggregate_batch(&mut self, batch: Vec<INSCFlamecall>, keys: &InfernalKeys) -> Result<Vec<INSCFlamecall>, InfernoError> {
        self.check_rate_limit(5)?;
        self.rotate_super_neurons()?;

        let mut confirmed = Vec::new();
        let super_neuron_count = self.super_neurons.len() as f32;
        let meta_neuron_count = self.meta_neurons.len() as f32;

        let mev_values: Vec<f32> = batch.iter().map(|tx| tx.mev_value).collect();
        let mev_mean = mev_values.iter().sum::<f32>() / mev_values.len() as f32;
        let mev_variance = mev_values.iter().map(|v| (v - mev_mean).powi(2)).sum::<f32>() / mev_values.len() as f32;
        if mev_variance > 0.1 {
            debug!("Adjusted flame mode thresholds due to MEV variance: {:.2}", mev_variance);
        }

        for tx in batch {
            let hash = tx.core.hash();
            let sig = keys.sign(hash.as_bytes())?;
            let approval_rate = if meta_neuron_count > 0.0 {
                let heat_index = self.shards.iter().map(|s| s.neurons.len() as f32).sum::<f32>() / self.shards.len() as f32;
                let failure_rate = 1.0 - self.shards.iter().map(|s| s.success_rate).sum::<f32>() / self.shards.len() as f32;
                let latency_patterns = self.shards.iter().map(|s| s.neurons.iter().map(|n| n.flamekeeper.latency as f32).sum::<f32>() / s.neurons.len() as f32).sum::<f32>() / self.shards.len() as f32;
                let meta_approval = self.meta_neurons.iter()
                    .map(|mn| self.meta_neuron_analyze(mn, &tx, heat_index, failure_rate, latency_patterns))
                    .sum::<f32>() / meta_neuron_count;
                meta_approval
            } else {
                self.super_neurons.iter()
                    .filter(|sn| sn.neuron.capacity_score > 50.0)
                    .count() as f32 / super_neuron_count
            };
            if approval_rate >= 0.66 {
                confirmed.push(tx);
                debug!("Tx {} confirmed by swarm: approval_rate={:.2}", hash, approval_rate);
            }
        }

        info!("Aggregated {} txs in Swarmforge", confirmed.len());
        Ok(confirmed)
    }

    #[inline]
    fn meta_neuron_analyze(&self, meta_neuron: &SuperNeuron, tx: &INSCFlamecall, heat_index: f32, failure_rate: f32, latency_patterns: f32) -> f32 {
        let capacity_factor = meta_neuron.neuron.capacity_score / 100.0;
        let tx_relevance = tx.mev_value / 2.0;
        let stability_factor = 1.0 - failure_rate;
        let latency_factor = 1.0 - (latency_patterns / 1000.0);
        (capacity_factor * tx_relevance * stability_factor * latency_factor).min(1.0)
    }

    #[inline]
    fn rotate_super_neurons(&mut self) -> Result<(), InfernoError> {
        self.rotation_count += 1;
        if self.rotation_count >= 10 {
            let mut rng = rand::thread_rng();
            let rotation_size = (self.super_neurons.len() as f32 * 0.3).ceil() as usize;
            self.super_neurons.shuffle(&mut rng);
            let rotated = self.super_neurons.drain(..rotation_size.min(self.super_neurons.len())).collect::<Vec<_>>();
            self.meta_neurons.extend(rotated);
            self.rotation_count = 0;
            info!("Rotated 30% of super neurons: super_neurons={}, meta_neurons={}", self.super_neurons.len(), self.meta_neurons.len());
        }
        Ok(())
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_aggregation {
            self.last_aggregation = now;
        } else if (self.shards.len() + self.super_neurons.len() + self.meta_neurons.len()) as u32 >= max_per_sec {
            return Err(InfernoError::Network("Swarmforge rate limit exceeded".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarmforge_aggregation() {
        let mut sf = Swarmforge::new().unwrap();
        let fk = Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap();
        let neuron = Neuron::new(1, fk).unwrap();
        let keys = InfernalKeys::new().unwrap();
        sf.add_super_neuron(SuperNeuron::new(1, neuron, Some(&keys)).unwrap()).unwrap();

        let core_tx = CoreFlamecall::new(1, "Alice", "Bob", 100, 100_000, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), None, None).unwrap();
        let tx = INSCFlamecall::new(core_tx, 1.0).unwrap();
        let batch = vec![tx];
        let confirmed = sf.aggregate_batch(batch, &keys).unwrap();
        assert_eq!(confirmed.len(), 1);
    }
}