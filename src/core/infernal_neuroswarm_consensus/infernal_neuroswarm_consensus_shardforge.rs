// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_shardforge.rs

use crate::core::infernal_types::infernal_types_flamekeeper::Flamekeeper;
use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::{Shard, Neuron};
use crate::core::error::InfernoError;
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::collections::HashMap;
use tokio::time::interval;

#[derive(Debug)]
pub struct Shardforge {
    shards: Vec<Shard>,
    min_shard_size: usize,
    max_shard_size: usize,
    last_assignment: u64,
    lazy_activation_threshold: f32,
    regional_tps: HashMap<String, f32>,
}

impl Shardforge {
    #[inline]
    pub fn new(min_shard_size: usize, max_shard_size: usize, lazy_activation_threshold: f32) -> Result<Self, InfernoError> {
        if min_shard_size >= max_shard_size || min_shard_size == 0 {
            return Err(InfernoError::Network("Invalid shard size constraints".into()));
        }
        if lazy_activation_threshold < 0.0 || lazy_activation_threshold > 1.0 {
            return Err(InfernoError::ParseError("Lazy activation threshold must be between 0 and 1".into()));
        }
        let sf = Self {
            shards: Vec::new(),
            min_shard_size,
            max_shard_size,
            last_assignment: 0,
            lazy_activation_threshold,
            regional_tps: HashMap::new(),
        };
        info!("Initialized Shardforge: min_size={}, max_size={}, lazy_threshold={}", min_shard_size, max_shard_size, lazy_activation_threshold);
        Ok(sf)
    }

    #[inline]
    pub fn assign_validators_to_shards(&mut self, validators: &[Flamekeeper], tps: f32) -> Result<(), InfernoError> {
        self.check_rate_limit(5)?;
        for validator in validators {
            let region = Self::get_region_from_location(validator.latitude, validator.longitude);
            let regional_tps = self.regional_tps.entry(region.clone()).or_insert(0.0);
            *regional_tps += tps / validators.len() as f32;

            let mut assigned = false;
            for shard in &mut self.shards {
                if shard.region == region && shard.neurons.len() < self.max_shard_size {
                    if validator.activity >= self.lazy_activation_threshold || *regional_tps > 1_000.0 {
                        shard.add_neuron(Neuron::new(validator.id, validator.clone())?)?;
                        assigned = true;
                        break;
                    }
                }
            }

            if !assigned && (*regional_tps > 1_000.0 || validator.activity >= self.lazy_activation_threshold) {
                let mut new_shard = Shard::new(self.shards.len() as u64 + 1, &region)?;
                new_shard.add_neuron(Neuron::new(validator.id, validator.clone())?)?;
                self.shards.push(new_shard);
            }
        }

        self.compress_shards()?;
        self.auto_fission_shards()?;
        self.fractal_fission(tps)?;
        self.rebind_shards()?;
        debug!("Assigned {} validators to {} shards with tps={}", validators.len(), self.shards.len(), tps);
        Ok(())
    }

    #[inline]
    fn compress_shards(&mut self) -> Result<(), InfernoError> {
        let mut i = 0;
        while i < self.shards.len() {
            let shard = &mut self.shards[i];
            shard.neurons.retain(|n| {
                if n.flamekeeper.activity < 0.1 {
                    debug!("Compressed neuron {} in shard {} with activity={}", n.id, shard.id, n.flamekeeper.activity);
                    false
                } else {
                    true
                }
            });
            if shard.neurons.len() < self.min_shard_size {
                let small_shard = self.shards.remove(i);
                for neuron in small_shard.neurons {
                    let mut reassigned = false;
                    for shard in &mut self.shards {
                        if shard.region == small_shard.region && shard.neurons.len() < self.max_shard_size {
                            shard.add_neuron(neuron)?;
                            reassigned = true;
                            break;
                        }
                    }
                    if !reassigned && self.shards.len() < self.max_shard_size {
                        let mut new_shard = Shard::new(self.shards.len() as u64 + 1, &small_shard.region)?;
                        new_shard.add_neuron(neuron)?;
                        self.shards.push(new_shard);
                    }
                }
            } else {
                i += 1;
            }
        }
        debug!("Compressed shards: total shards={}", self.shards.len());
        Ok(())
    }

    #[inline]
    fn auto_fission_shards(&mut self) -> Result<(), InfernoError> {
        let mut new_shards = Vec::new();
        let mut i = 0;
        while i < self.shards.len() {
            let shard = &self.shards[i];
            let failure_rate = 1.0 - shard.success_rate;
            let latency = shard.neurons.iter().map(|n| n.flamekeeper.latency as f32).sum::<f32>() / shard.neurons.len() as f32;
            if failure_rate > 0.3 && latency > 50.0 {
                let large_shard = self.shards.remove(i);
                let half = large_shard.neurons.len() / 2;
                let mut shard1 = Shard::new(large_shard.id, &large_shard.region)?;
                let mut shard2 = Shard::new(self.shards.len() as u64 + new_shards.len() as u64 + 1, &large_shard.region)?;
                shard1.neurons = large_shard.neurons[..half].to_vec();
                shard2.neurons = large_shard.neurons[half..].to_vec();
                new_shards.push(shard1);
                new_shards.push(shard2);
                info!("Auto-fissioned shard {} due to failure_rate={:.2}, latency={:.2}", large_shard.id, failure_rate, latency);
            } else {
                i += 1;
            }
        }
        self.shards.extend(new_shards);
        Ok(())
    }

    #[inline]
    fn fractal_fission(&mut self, tps: f32) -> Result<(), InfernoError> {
        if tps > 1_000_000.0 {
            let mut new_shards = Vec::new();
            let mut i = 0;
            while i < self.shards.len() {
                let shard = self.shards.remove(i);
                let quarter = shard.neurons.len() / 4;
                if quarter > self.min_shard_size {
                    for j in 0..4 {
                        let start = j * quarter;
                        let end = if j == 3 { shard.neurons.len() } else { (j + 1) * quarter };
                        let mut new_shard = Shard::new(self.shards.len() as u64 + new_shards.len() as u64 + 1, &shard.region)?;
                        new_shard.neurons = shard.neurons[start..end].to_vec();
                        new_shards.push(new_shard);
                    }
                    info!("Fractal-fissioned shard {} into 4 parts due to tps={}", shard.id, tps);
                } else {
                    self.shards.insert(i, shard);
                    i += 1;
                }
            }
            self.shards.extend(new_shards);
        }
        Ok(())
    }

    #[inline]
    fn rebind_shards(&mut self) -> Result<(), InfernoError> {
        let mut i = 0;
        while i < self.shards.len() - 1 {
            let shard_a = &self.shards[i];
            let mut j = i + 1;
            while j < self.shards.len() {
                let shard_b = &self.shards[j];
                let interlinking = Self::tx_interlinking(shard_a, shard_b);
                if interlinking > 0.7 {
                    let merged_shard = self.shards.remove(j);
                    self.shards[i].neurons.extend(merged_shard.neurons);
                    info!("Temporarily merged shards {} and {} due to interlinking={:.2}", shard_a.id, merged_shard.id, interlinking);
                    continue;
                }
                j += 1;
            }
            i += 1;
        }
        Ok(())
    }

    #[inline]
    fn tx_interlinking(shard_a: &Shard, shard_b: &Shard) -> f32 {
        if shard_a.region == shard_b.region {
            let overlap = shard_a.neurons.len().min(shard_b.neurons.len()) as f32 / shard_a.neurons.len().max(shard_b.neurons.len()) as f32;
            overlap * 0.9
        } else {
            0.0
        }
    }

    fn get_region_from_location(lat: f64, lon: f64) -> String {
        if lat > 0.0 && lon > 0.0 { "NE" }
        else if lat > 0.0 && lon <= 0.0 { "NW" }
        else if lat <= 0.0 && lon > 0.0 { "SE" }
        else { "SW" }.to_string()
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_assignment {
            self.last_assignment = now;
        } else if self.shards.len() as u32 >= max_per_sec {
            return Err(InfernoError::Network("Shardforge assignment rate limit exceeded".into()));
        }
        Ok(())
    }

    pub async fn self_heal(&mut self) -> Result<(), InfernoError> {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            self.compress_shards()?;
            self.auto_fission_shards()?;
            self.rebind_shards()?;
            info!("Self-healing cycle completed: shards={}", self.shards.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shardforge_full_features() {
        let mut sf = Shardforge::new(2, 4, 0.1).unwrap();
        let mut validators = Vec::new();
        for i in 1..=10 {
            let mut fk = Flamekeeper::new(i, 500, 50.0 * (i % 2) as f64, 10.0 * (i % 3) as f64, 6.0, 4.0, 50.0).unwrap();
            fk.activity = if i <= 8 { 0.5 } else { 0.05 };
            validators.push(fk);
        }
        assert!(sf.assign_validators_to_shards(&validators, 1_500_000.0).is_ok());
        assert!(sf.shards.len() > 2); // Fractal Fission
        assert!(sf.shards.iter().all(|s| s.neurons.len() >= 2 && s.neurons.len() <= 4)); // Compression und Auto-Fission
    }
}