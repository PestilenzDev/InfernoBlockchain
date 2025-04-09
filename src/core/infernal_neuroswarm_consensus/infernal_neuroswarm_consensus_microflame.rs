// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_microflame.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::{Shard, Neuron, INSCFlamecall};
use crate::core::error::InfernoError;
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::collections::HashMap;
use tokio::sync::mpsc;
use sha2::{Sha256, Digest};
use tch::{nn, Tensor, Device, Kind};
use libp2p::gossipsub::{Gossipsub, GossipsubEvent, IdentTopic};
use futures::StreamExt;

#[derive(Debug)]
pub struct Microflame {
    pub shard: Shard,
    task_min_capacity: f32,
    last_task_assignment: u64,
    bypass_queue: Vec<INSCFlamecall>,
    clusters: HashMap<String, Vec<Neuron>>,
    feedback_tx: mpsc::Sender<(u64, f32, f32)>,
    feedback_rx: mpsc::Receiver<(u64, f32, f32)>,
    geo_nn: nn::Sequential,
    gossipsub: Gossipsub,
}

impl Microflame {
    #[inline]
    pub fn new(shard: Shard, complexity_factor: f32, gossipsub: Gossipsub) -> Result<Self, InfernoError> {
        if complexity_factor <= 0.0 {
            return Err(InfernoError::Network("Complexity factor must be positive".into()));
        }
        let tps = shard.neurons.len() as f32 * 10.0;
        let latency_threshold = 0.05;
        let task_min_capacity = complexity_factor * (tps / 1_000.0) * latency_threshold;
        let (feedback_tx, feedback_rx) = mpsc::channel(100);
        let vs = nn::VarStore::new(Device::Cpu);
        let geo_nn = nn::seq()
            .add(nn::linear(&vs.root(), 1, 8, Default::default()))
            .add_fn(|xs| xs.relu())
            .add(nn::linear(&vs.root(), 8, 1, Default::default()));
        let mf = Self {
            shard,
            task_min_capacity,
            last_task_assignment: 0,
            bypass_queue: Vec::new(),
            clusters: HashMap::new(),
            feedback_tx,
            feedback_rx,
            geo_nn,
            gossipsub,
        };
        info!("Created Microflame for Shard {} with task_min_capacity={}", mf.shard.id, task_min_capacity);
        Ok(mf)
    }

    #[inline]
    pub async fn assign_task(&mut self, tx: INSCFlamecall) -> Result<(), InfernoError> {
        self.check_rate_limit(100)?;
        let total_capacity = self.shard.neurons.iter().map(|n| n.capacity_score).sum::<f32>();
        let strong_nodes: Vec<&Neuron> = self.shard.neurons.iter()
            .filter(|n| n.capacity_score >= self.task_min_capacity)
            .collect();

        let cluster_key = Self::compute_cluster_key(&tx, &self.shard);
        let cluster = self.clusters.entry(cluster_key.clone()).or_insert_with(Vec::new);
        if let Some(node) = strong_nodes.first() {
            cluster.push(node.clone());
        }

        let total_task_complexity = tx.mev_value + 1.0;
        if strong_nodes.is_empty() {
            self.tier0_bypass(&tx)?;
        } else {
            for node in strong_nodes {
                let workload = if node.capacity_score >= self.task_min_capacity {
                    total_task_complexity
                } else {
                    total_task_complexity * (node.capacity_score / total_capacity).max(0.1)
                };
                let latency = node.flamekeeper.latency as f32;
                debug!("Assigned task {} to node {} in shard {} with workload={:.2}, latency={:.2}", tx.core.id, node.id, self.shard.id, workload, latency);
                self.feedback_tx.send((node.id, workload, latency)).await?;
            }
            info!("Microflame {} assigned task to {} strong nodes", self.shard.id, strong_nodes.len());
        }

        let local_feedback = self.shard.success_rate;
        let mut neighbor_feedback = 0.0;
        while let Some((_, workload, _)) = self.feedback_rx.recv().timeout(Duration::from_millis(10)).await? {
            neighbor_feedback += workload;
        }
        neighbor_feedback /= self.shard.neurons.len() as f32 + 1.0;
        let diffusion_weight = 0.8 * local_feedback + 0.2 * neighbor_feedback;
        debug!("Diffusion weight for shard {}: {:.2}", self.shard.id, diffusion_weight);

        let latency_clusters = self.shard.neurons.iter().map(|n| n.flamekeeper.latency as f32).collect::<Vec<_>>();
        let latency_input = Tensor::of_slice(&[latency_clusters.iter().sum::<f32>() / latency_clusters.len() as f32])
            .view([1, 1])
            .to_kind(Kind::Float);
        let latency_weight = self.geo_nn.forward(&latency_input).double_value(&[0]) as f32;
        debug!("Geo-feedback latency weight for shard {}: {:.2}", self.shard.id, latency_weight);

        let sync_delta = self.compute_sync_delta().await?;
        if sync_delta > 0.1 {
            self.propagate_with_pheromones(sync_delta).await?;
        }
        debug!("Anti-entropy sync delta for shard {}: {:.2}", self.shard.id, sync_delta);

        Ok(())
    }

    #[inline]
    fn compute_cluster_key(tx: &INSCFlamecall, shard: &Shard) -> String {
        let mut hasher = Sha256::new();
        hasher.update(tx.mev_value.to_be_bytes());
        hasher.update(tx.core.timestamp.to_be_bytes());
        hasher.update(shard.region.as_bytes());
        hex::encode(hasher.finalize())
    }

    #[inline]
    async fn compute_sync_delta(&self) -> Result<f32, InfernoError> {
        let state_hash = self.shard.neurons.iter().fold(Sha256::new(), |mut h, n| {
            h.update(n.id.to_be_bytes());
            h
        }).finalize();
        let neighbor_hash = self.fetch_neighbor_state().await?;
        let delta = state_hash.iter().zip(neighbor_hash.iter()).map(|(&a, &b)| (a ^ b) as f32).sum::<f32>() / 256.0;
        Ok(delta)
    }

    #[inline]
    async fn fetch_neighbor_state(&self) -> Result<Vec<u8>, InfernoError> {
        let topic = IdentTopic::new(format!("insc_sync_{}", self.shard.id));
        let state_request = self.shard.id.to_be_bytes().to_vec();
        self.gossipsub.publish(topic.clone(), state_request)?;
        
        // Realistischer Abruf von Nachbarn
        let mut subscription = self.gossipsub.subscribe(&topic)?;
        let mut neighbor_hash = None;
        if let Some(msg) = subscription.next().await {
            neighbor_hash = Some(msg.data);
        }
        neighbor_hash.ok_or(InfernoError::Network("No neighbor response".into()))
    }

    #[inline]
    async fn propagate_with_pheromones(&self, sync_delta: f32) -> Result<(), InfernoError> {
        let topic = IdentTopic::new(format!("insc_sync_{}", self.shard.id));
        let pheromone_data = format!("delta:{:.2}", sync_delta).into_bytes();
        self.gossipsub.publish(topic, pheromone_data)?;
        debug!("Propagated sync delta {:.2} with pheromones for shard {}", sync_delta, self.shard.id);
        Ok(())
    }

    #[inline]
    fn tier0_bypass(&mut self, tx: &INSCFlamecall) -> Result<(), InfernoError> {
        if self.bypass_queue.len() >= 1000 {
            return Err(InfernoError::Network("Tier-0 bypass queue full".into()));
        }
        self.bypass_queue.push(tx.clone());
        debug!("Tx {} added to Tier-0 bypass queue in shard {}", tx.core.id, self.shard.id);
        if self.bypass_queue.len() > 100 {
            self.process_bypass_queue()?;
        }
        Ok(())
    }

    #[inline]
    fn process_bypass_queue(&mut self) -> Result<(), InfernoError> {
        let bypass_batch = self.bypass_queue.drain(..).collect::<Vec<_>>();
        for tx in bypass_batch {
            info!("Processed tx {} via Tier-0 bypass in shard {}", tx.core.id, self.shard.id);
        }
        Ok(())
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_task_assignment {
            self.last_task_assignment = now;
        } else if self.shard.neurons.len() as u32 >= max_per_sec {
            return Err(InfernoError::Network("Microflame task rate limit exceeded".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::gossipsub::{GossipsubConfigBuilder, Gossipsub};
    use libp2p::identity::Keypair;

    #[tokio::test]
    async fn test_microflame_task_assignment() {
        let keypair = Keypair::generate_ed25519();
        let gossipsub = Gossipsub::new(keypair.into(), GossipsubConfigBuilder::default().build().unwrap()).unwrap();
        let mut shard = Shard::new(1, "EU").unwrap();
        let fk = Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap();
        shard.add_neuron(Neuron::new(1, fk).unwrap()).unwrap();
        let mut mf = Microflame::new(shard, 50.0, gossipsub).unwrap();
        let core_tx = CoreFlamecall::new(1, "Alice", "Bob", 100, 100_000, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), None, None).unwrap();
        let tx = INSCFlamecall::new(core_tx, 1.0).unwrap();
        assert!(mf.assign_task(tx).await.is_ok());
        assert_eq!(mf.clusters.len(), 1);
    }
}