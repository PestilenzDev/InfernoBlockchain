// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_geofire.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::Shard;
use crate::core::infernal_types::infernal_types_flamekeeper::Flamekeeper;
use crate::core::error::InfernoError;
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Geofire {
    shards: Vec<Shard>,
    last_optimization: u64,
    frozen_shards: Vec<u64>,
}

impl Geofire {
    #[inline]
    pub fn new(shards: Vec<Shard>) -> Result<Self, InfernoError> {
        let gf = Self {
            shards,
            last_optimization: 0,
            frozen_shards: Vec::new(),
        };
        info!("Initialized Geofire with {} shards", gf.shards.len());
        Ok(gf)
    }

    #[inline]
    pub fn optimize_latency(&mut self, validators: &[Flamekeeper]) -> Result<(), InfernoError> {
        self.check_rate_limit(5)?;
        for shard in &mut self.shards {
            if self.frozen_shards.contains(&shard.id) {
                continue;
            }
            let mut total_latency = 0.0;
            let mut count = 0;
            for neuron in &mut shard.neurons {
                if let Some(v) = validators.iter().find(|v| v.id == neuron.id) {
                    neuron.flamekeeper.update_latency(v.latency)?;
                    total_latency += v.latency as f32;
                    count += 1;
                }
            }
            if count > 0 {
                let avg_latency = total_latency / count as f32;
                shard.success_rate = if avg_latency < 30.0 { 0.9 } else { 0.8 - (avg_latency / 1000.0).min(0.8) };
                debug!("Optimized Shard {}: avg_latency={:.2}ms, success_rate={:.2}", shard.id, avg_latency, shard.success_rate);
            }
        }
        self.heat_mapping()?;
        info!("Optimized latency for {} shards", self.shards.len());
        Ok(())
    }

    #[inline]
    fn heat_mapping(&mut self) -> Result<(), InfernoError> {
        let mut heat_indices = Vec::new();
        for shard in &self.shards {
            let heat_index = shard.neurons.iter().map(|n| n.flamekeeper.activity).sum::<f32>() / shard.neurons.len() as f32;
            heat_indices.push(heat_index);
        }
        let avg_heat_index = heat_indices.iter().sum::<f32>() / heat_indices.len() as f32;
        let variance = heat_indices.iter().map(|h| (h - avg_heat_index).powi(2)).sum::<f32>() / heat_indices.len() as f32;

        if avg_heat_index > 0.8 && variance < 0.05 {
            for shard in &self.shards {
                if !self.frozen_shards.contains(&shard.id) {
                    self.frozen_shards.push(shard.id);
                    info!("Froze shard {} structure: avg_heat_index={:.2}, variance={:.2}", shard.id, avg_heat_index, variance);
                }
            }
        }
        Ok(())
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_optimization {
            self.last_optimization = now;
        } else if max_per_sec == 0 {
            return Err(InfernoError::Network("Geofire rate limit exceeded".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geofire_heat_mapping() {
        let mut shard = Shard::new(1, "EU").unwrap();
        let fk = Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap();
        shard.add_neuron(Neuron::new(1, fk.clone()).unwrap()).unwrap();
        let mut gf = Geofire::new(vec![shard]).unwrap();
        let mut validators = vec![fk];
        validators[0].activity = 0.9;
        assert!(gf.optimize_latency(&validators).is_ok());
        assert_eq!(gf.frozen_shards.len(), 1);
    }
}