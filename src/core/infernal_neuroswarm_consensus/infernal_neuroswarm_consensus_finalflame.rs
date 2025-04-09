// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_finalflame.rs

use crate::core::infernal_types::infernal_keys::InfernalKeys;
use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::{INSCEmberblock, SuperNeuron};
use crate::core::error::InfernoError;
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tch::{nn, Tensor, Device, Kind};

#[derive(Debug)]
pub struct Finalflame {
    super_neurons: Vec<SuperNeuron>,
    approval_threshold: f32,
    last_finalization: u64,
    epoch_duration: Duration,
    current_epoch: u64,
    nn: nn::Sequential, // Für block_interval Vorhersage
}

impl Finalflame {
    #[inline]
    pub fn new(super_neurons: Vec<SuperNeuron>, approval_threshold: f32, epoch_duration_secs: u64) -> Result<Self, InfernoError> {
        if approval_threshold < 0.0 || approval_threshold > 1.0 {
            return Err(InfernoError::ParseError("Approval threshold must be between 0 and 1".into()));
        }
        if epoch_duration_secs == 0 {
            return Err(InfernoError::Network("Epoch duration must be positive".into()));
        }
        let vs = nn::VarStore::new(Device::Cpu);
        let nn = nn::seq()
            .add(nn::linear(&vs.root(), 3, 8, Default::default()))
            .add_fn(|xs| xs.relu())
            .add(nn::linear(&vs.root(), 8, 1, Default::default()));
        let ff = Self {
            super_neurons,
            approval_threshold,
            last_finalization: 0,
            epoch_duration: Duration::from_secs(epoch_duration_secs),
            current_epoch: 0,
            nn,
        };
        info!("Initialized Finalflame with {} super neurons, threshold={}, epoch_duration={}s", ff.super_neurons.len(), approval_threshold, epoch_duration_secs);
        Ok(ff)
    }

    #[inline]
    pub fn finalize_global(&mut self, block: &mut INSCEmberblock, keys: &InfernalKeys) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        self.update_epoch()?;

        let load_metrics = vec![
            self.super_neurons.len() as f32,
            block.core.transactions.len() as f32,
            self.super_neurons.iter().map(|sn| sn.neuron.flamekeeper.latency as f32).sum::<f32>() / self.super_neurons.len() as f32,
        ];
        let block_interval = self.predict_block_interval(&load_metrics)?;
        let approval_count = self.super_neurons.iter()
            .filter(|sn| sn.neuron.capacity_score > 50.0)
            .count() as f32;
        let approval_rate = approval_count / self.super_neurons.len() as f32;
        block.core.update_approval(approval_rate)?;

        if approval_rate >= self.approval_threshold {
            let hash = block.core.hash();
            block.core.signature = Some(keys.sign(hash.as_bytes())?);
            info!("Finalized block {} in epoch {} with approval rate={:.2}", block.core.height, self.current_epoch, approval_rate);
        } else if SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() - block.core.timestamp > (self.epoch_duration.as_secs() as f32 * 0.05).ceil() as u64 {
            self.fallback_aggregate(block, keys)?;
        } else {
            return Err(InfernoError::Network(format!("Insufficient approval for finality: {:.2}", approval_rate)));
        }

        // Route-Priority
        let latency = self.super_neurons.iter().map(|sn| sn.neuron.flamekeeper.latency as f32).sum::<f32>() / self.super_neurons.len() as f32;
        let route_priority = (-latency / 30.0).exp();
        debug!("Route priority for block {}: {:.2}", block.core.height, route_priority);

        Ok(())
    }

    #[inline]
    fn predict_block_interval(&self, load_metrics: &[f32]) -> Result<f32, InfernoError> {
        let inputs = Tensor::of_slice(load_metrics)
            .view([1, 3])
            .to_kind(Kind::Float);
        let interval = self.nn.forward(&inputs).double_value(&[0]) as f32;
        Ok(interval.max(0.05)) // Mindestens 0.05s
    }

    #[inline]
    fn update_epoch(&mut self) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let new_epoch = now / self.epoch_duration.as_secs();
        if new_epoch > self.current_epoch {
            self.current_epoch = new_epoch;
            debug!("Advanced to epoch {}", self.current_epoch);
        }
        Ok(())
    }

    #[inline]
    fn fallback_aggregate(&mut self, block: &mut INSCEmberblock, keys: &InfernalKeys) -> Result<(), InfernoError> {
        let high_capacity_nodes = self.super_neurons.iter()
            .filter(|sn| sn.neuron.capacity_score >= 100.0)
            .count();
        if high_capacity_nodes > 0 {
            let hash = block.core.hash();
            block.core.signature = Some(keys.sign(hash.as_bytes())?);
            block.core.update_approval(0.51)?; // Minimale Finalität
            info!("Fallback aggregation for block {} in epoch {} with {} high-capacity nodes", block.core.height, self.current_epoch, high_capacity_nodes);
            Ok(())
        } else {
            Err(InfernoError::Network("No high-capacity nodes for fallback".into()))
        }
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_finalization {
            self.last_finalization = now;
        } else if max_per_sec == 0 {
            return Err(InfernoError::Network("Finalflame rate limit exceeded".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finalflame_finalization() {
        let fk = Flamekeeper::new(1, 500, 50.0, 10.0, 16.0, 32.0, 100.0).unwrap();
        let neuron = Neuron::new(1, fk).unwrap();
        let super_neuron = SuperNeuron::new(1, neuron, None).unwrap();
        let mut ff = Finalflame::new(vec![super_neuron], 0.51, 10).unwrap();
        let core_tx = CoreFlamecall::new(1, "Alice", "Bob", 100, 100_000, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), None, None).unwrap();
        let core_block = CoreEmberblock::new(1, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef", vec![core_tx], None).unwrap();
        let mut block = INSCEmberblock::new(core_block, vec![1], 1).unwrap();
        let keys = InfernalKeys::new().unwrap();

        assert!(ff.finalize_global(&mut block, &keys).is_ok());
        assert_eq!(ff.current_epoch, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() / 10);
        assert!(block.core.signature.is_some());
    }
}