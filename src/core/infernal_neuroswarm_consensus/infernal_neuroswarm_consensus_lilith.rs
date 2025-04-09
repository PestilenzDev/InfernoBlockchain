// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_lilith.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::{INSCFlamecall, Shard};
use crate::core::error::InfernoError;
use log::{debug, info, warn};
use tch::{nn, Tensor, Device, Kind, nn::Optimizer};
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::collections::{HashMap, HashSet};
use tokio::spawn;
use libp2p::gossipsub::{Gossipsub, GossipsubEvent, IdentTopic};
use sha2::{Sha256, Digest};

#[derive(Debug)]
pub struct Lilith {
    nn: nn::Sequential,
    optimizer: Optimizer,
    use_gpu: bool,
    chaos_budget: f32,
    last_prediction: u64,
    memory_layer: HashMap<String, (f32, f32, f32, f32)>,
    base_limit: usize,
    gossipsub: Gossipsub,
    spiked_neurons: HashMap<u64, Instant>,
    shards: Vec<Shard>,
    decay_factor: f32,
}

impl Lilith {
    #[inline]
    pub fn new(use_gpu: bool, base_limit: usize, gossipsub: Gossipsub, shards: Vec<Shard>) -> Result<Self, InfernoError> {
        let device = if use_gpu && tch::Cuda::is_available() { Device::Cuda(0) } else { Device::Cpu };
        let vs = nn::VarStore::new(device);
        let nn = nn::seq()
            .add(nn::linear(&vs.root(), 6, 16, Default::default()))
            .add_fn(|xs| xs.relu())
            .add(nn::linear(&vs.root(), 16, 4, Default::default()));
        let optimizer = nn::Adam::default().build(&vs, 0.01)?;
        let lilith = Self {
            nn,
            optimizer,
            use_gpu,
            chaos_budget: 0.1,
            last_prediction: 0,
            memory_layer: HashMap::new(),
            base_limit,
            gossipsub,
            spiked_neurons: HashMap::new(),
            shards,
            decay_factor: 0.99999,
        };
        info!("Initialized Lilith with base_limit={} and {} shards, GPU={}", base_limit, lilith.shards.len(), use_gpu);
        Ok(lilith)
    }

    #[inline]
    pub fn predict(&mut self, latency: f32, stake: f32, tps: f32, mev: f32, success_rate: f32) -> Result<(f32, f32, f32, f32), InfernoError> {
        self.check_rate_limit(100)?;
        let num_regions = self.shards.iter().map(|s| s.region.clone()).collect::<HashSet<_>>().len() as f32;
        let key = format!("{:.2}-{:.2}-{:.2}-{:.2}-{:.2}-{:.2}", latency, stake, tps, mev, success_rate, num_regions);
        if let Some(cached) = self.memory_layer.get(&key) {
            debug!("Lilith used cached prediction for key {}", key);
            return Ok(*cached);
        }

        let inputs = Tensor::of_slice(&[latency, stake, tps, mev, success_rate, num_regions])
            .view([1, 6])
            .to_device(if self.use_gpu { Device::Cuda(0) } else { Device::Cpu })
            .to_kind(Kind::Float);
        let outputs = self.nn.forward(&inputs).squeeze();
        let neuron_score = outputs.double_value(&[0]) as f32;
        let shard_size = (outputs.double_value(&[1]) as f32).max(50.0).min(500.0);
        let mev_mode = (outputs.double_value(&[2]) as f32).clamp(0.0, 1.0);
        let adjustment_factor = (outputs.double_value(&[3]) as f32).max(0.5).min(2.0);
        self.memory_layer.insert(key.clone(), (neuron_score, shard_size, mev_mode, adjustment_factor));
        debug!("Lilith predicted: neuron_score={:.2}, shard_size={:.2}, mev_mode={:.2}, adjustment_factor={:.2}", neuron_score, shard_size, mev_mode, adjustment_factor);
        Ok((neuron_score, shard_size, mev_mode, adjustment_factor))
    }

    #[inline]
    pub async fn filter_spam(&mut self, batch: Vec<INSCFlamecall>) -> Result<Vec<INSCFlamecall>, InfernoError> {
        self.check_rate_limit(50)?;
        let tx_rate = batch.len() as f32 / 0.1;
        let avg_success = batch.iter().map(|tx| tx.core.success_rate).sum::<f32>() / batch.len() as f32;
        let latency_spike = batch.iter().map(|tx| (SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as f32 - tx.core.timestamp as f32) * 1000.0).sum::<f32>() / batch.len() as f32;
        let mev_value = batch.iter().map(|tx| tx.mev_value).sum::<f32>() / batch.len() as f32;
        let (neuron_score, _, _, adjustment_factor) = self.predict(latency_spike, 1000.0, tx_rate, mev_value, avg_success)?;

        let tx_limit = (self.base_limit as f32 * adjustment_factor) as usize;
        let mut filtered = Vec::with_capacity(tx_limit.min(batch.len()));

        for tx in batch.into_iter().take(tx_limit) {
            let latency = (SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as f32 - tx.core.timestamp as f32) * 1000.0;
            if tx.core.success_rate >= adjustment_factor {
                if latency < 10.0 && tx.core.success_rate > 0.98 {
                    self.spiked_neurons.insert(tx.core.id, Instant::now());
                    debug!("Spiked neuron {} for tx {}: latency={:.2}ms, success_rate={:.2}", tx.core.id, tx.core.id, latency, tx.core.success_rate);
                }

                filtered.push(tx.clone());

                let trust_score = tx.core.success_rate;
                let info_relevance = tx.mev_value / 2.0;
                if trust_score > 0.9 && info_relevance > 0.5 {
                    let tx_digest = Self::compute_tx_digest(&tx);
                    let topic = IdentTopic::new("insc_gossip");
                    let gossipsub_clone = self.gossipsub.clone();
                    spawn(async move {
                        if let Err(e) = gossipsub_clone.publish(topic, tx_digest) {
                            warn!("Failed to propagate tx digest: {}", e);
                        } else {
                            debug!("Propagated tx digest {} to neighbor shards", hex::encode(&tx_digest[..8]));
                        }
                    });
                }
            }
        }

        self.spiked_neurons.retain(|_, start| Instant::now().duration_since(*start) < Duration::from_secs(30));
        self.update_weights(filtered.len() as f32, avg_success, mev_value);
        info!("Filtered {} txs, kept {}", batch.len(), filtered.len());
        Ok(filtered)
    }

    #[inline]
    fn compute_tx_digest(tx: &INSCFlamecall) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(tx.core.id.to_be_bytes());
        hasher.update(tx.core.hash().as_bytes());
        hasher.finalize().to_vec()
    }

    #[inline]
    pub fn adjust_chaos_budget(&mut self, system_load: f32) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        self.chaos_budget = 0.05_f32.max(0.2_f32.min(0.2 * (1.0 - system_load)));
        debug!("Adjusted chaos budget to {}", self.chaos_budget);
        Ok(())
    }

    #[inline]
    fn update_weights(&mut self, tx_count: f32, success_rate: f32, mev_value: f32) {
        let reward_spam = if tx_count < self.base_limit as f32 { 1.0 } else { -1.0 };
        let reward_success = success_rate;
        let reward_mev = mev_value / 100.0; // Beispielhafte Skalierung
        let reward_chaos = self.chaos_budget;
        let weight_adjust = 0.01 * (reward_spam + reward_success + reward_mev + reward_chaos);
        
        // Gewichte anpassen und Decay anwenden
        let inputs = Tensor::ones(&[1, 6], (Kind::Float, if self.use_gpu { Device::Cuda(0) } else { Device::Cpu }));
        let outputs = self.nn.forward(&inputs);
        let loss = outputs.sum(Kind::Float) * weight_adjust;
        self.optimizer.backward_step(&loss);
        
        // Decay anwenden
        let decay = Tensor::of_slice(&[self.decay_factor]).to_device(if self.use_gpu { Device::Cuda(0) } else { Device::Cpu });
        self.nn.iter_mut().for_each(|param| {
            param.data().mul_(&decay);
        });
        
        debug!("Updated weights with adjustment={:.4}, applied decay={}", weight_adjust, self.decay_factor);
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_prediction {
            self.last_prediction = now;
        } else if max_per_sec == 0 {
            return Err(InfernoError::Network("Lilith rate limit exceeded".into()));
        }
        Ok(())
    }

    pub fn update_shards(&mut self, shards: Vec<Shard>) {
        self.shards = shards;
        debug!("Updated Lilith with {} shards", self.shards.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::infernal_types::infernal_types_flamecall::Flamecall;
    use crate::core::infernal_types::infernal_types_flamekeeper::Flamekeeper;
    use libp2p::gossipsub::{GossipsubConfigBuilder, Gossipsub};
    use libp2p::identity::Keypair;

    #[tokio::test]
    async fn test_lilith_spam_filter() {
        let keypair = Keypair::generate_ed25519();
        let gossipsub = Gossipsub::new(keypair.into(), GossipsubConfigBuilder::default().build().unwrap()).unwrap();
        let mut shard = Shard::new(1, "EU").unwrap();
        shard.add_neuron(Neuron::new(1, Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap()).unwrap()).unwrap();
        let mut lilith = Lilith::new(false, 100, gossipsub, vec![shard]).unwrap();
        let mut batch = Vec::new();
        for i in 1..=200 {
            let core_tx = Flamecall::new(i, "Alice", "Bob", 100, 100_000, 0, None, None).unwrap();
            let mut tx = INSCFlamecall::new(core_tx, 1.0).unwrap();
            tx.core.success_rate = if i % 2 == 0 { 0.99 } else { 0.4 };
            batch.push(tx);
        }
        let filtered = lilith.filter_spam(batch).await.unwrap();
        assert!(filtered.len() <= 100);
        assert!(filtered.iter().all(|tx| tx.core.success_rate >= 0.5));
    }
}