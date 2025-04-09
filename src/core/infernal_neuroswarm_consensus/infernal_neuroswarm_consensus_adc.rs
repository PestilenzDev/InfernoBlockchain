// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_adc.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::Shard;
use crate::core::error::InfernoError;
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct ADC {
    shards: Vec<Shard>,
    avg_latency: f32,
    current_tps: f32,
    validator_count: usize,
    shard_size_min: usize,
    shard_size_max: usize,
    last_adjustment: u64,
}

impl ADC {
    #[inline]
    pub fn new(shards: Vec<Shard>, avg_latency: f32, current_tps: f32, validator_count: usize) -> Result<Self, InfernoError> {
        if avg_latency < 0.0 || current_tps < 0.0 {
            return Err(InfernoError::ParseError("Latency and TPS must be non-negative".into()));
        }
        let shard_size_min = 50.max((5.0 + 10.0 * (current_tps / 100_000.0)) as usize).min(500);
        let adc = Self {
            shards,
            avg_latency,
            current_tps,
            validator_count,
            shard_size_min,
            shard_size_max: shard_size_min * 2,
            last_adjustment: 0,
        };
        info!("Initialized ADC: shards={}, latency={:.2}ms, tps={:.2}, shard_size_min={}", adc.shards.len(), avg_latency, current_tps, shard_size_min);
        Ok(adc)
    }

    #[inline]
    pub fn update_parameters(&mut self, avg_latency: f32, current_tps: f32, validator_count: usize) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        if avg_latency < 0.0 || current_tps < 0.0 {
            return Err(InfernoError::ParseError("Latency and TPS must be non-negative".into()));
        }
        self.avg_latency = avg_latency;
        self.current_tps = current_tps;
        self.validator_count = validator_count;

        self.shard_size_min = 50.max((5.0 + 10.0 * (self.current_tps / 100_000.0)) as usize).min(500);
        self.shard_size_max = self.shard_size_min * 2;
        debug!("Updated ADC parameters: min_size={}, max_size={}", self.shard_size_min, self.shard_size_max);
        Ok(())
    }

    #[inline]
    pub fn adjust_shard_thresholds(&mut self) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        for shard in &mut self.shards {
            let neuron_count = shard.neurons.len();
            if neuron_count < self.shard_size_min {
                return Err(InfernoError::Network(format!(
                    "Shard {} has too few neurons: {} (min: {})", shard.id, neuron_count, self.shard_size_min
                )));
            }
            if neuron_count > self.shard_size_max {
                return Err(InfernoError::Network(format!(
                    "Shard {} has too many neurons: {} (max: {})", shard.id, neuron_count, self.shard_size_max
                )));
            }
        }
        info!("Adjusted shard thresholds for {} shards", self.shards.len());
        Ok(())
    }

    #[inline]
    pub fn apply_duty_cycling(&mut self, predicted_tps: f32) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        for shard in &mut self.shards {
            for neuron in &mut shard.neurons {
                if predicted_tps < 10_000.0 && neuron.flamekeeper.activity < 0.05 {
                    neuron.flamekeeper.standby = true;
                    debug!("Neuron {} in shard {} set to standby: tps={:.2}, activity={:.2}", neuron.id, shard.id, predicted_tps, neuron.flamekeeper.activity);
                } else {
                    neuron.flamekeeper.standby = false;
                }
            }
        }
        let standby_count = self.shards.iter().flat_map(|s| s.neurons.iter()).filter(|n| n.flamekeeper.standby).count();
        let total_count = self.shards.iter().flat_map(|s| s.neurons.iter()).count();
        info!("Duty cycling applied: {} of {} neurons on standby", standby_count, total_count);
        Ok(())
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_adjustment {
            self.last_adjustment = now;
        } else if max_per_sec == 0 {
            return Err(InfernoError::Network("ADC rate limit exceeded".into()));
        }
        Ok(())
    }

    pub fn get_shards(&self) -> &[Shard] {
        &self.shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc_duty_cycling() {
        let mut shard = Shard::new(1, "EU").unwrap();
        for i in 0..50 {
            let mut fk = Flamekeeper::new(i, 1000, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap();
            fk.activity = if i % 2 == 0 { 0.04 } else { 0.06 };
            shard.add_neuron(Neuron::new(i, fk).unwrap()).unwrap();
        }
        let mut adc = ADC::new(vec![shard], 50.0, 5000.0, 50).unwrap();
        adc.apply_duty_cycling(5000.0).unwrap();
        assert!(adc.get_shards()[0].neurons.iter().filter(|n| n.flamekeeper.standby).count() > 0);
    }
}