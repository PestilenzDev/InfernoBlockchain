// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_chaos.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::Shard;
use crate::core::error::InfernoError;
use log::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};
use libp2p::gossipsub::{Gossipsub, GossipsubEvent, IdentTopic};
use tokio::time::{sleep, Duration};

#[derive(Debug)]
pub struct Chaos {
    chaos_budget: f32,
    last_chaos: u64,
    partition_rate: f32,
    latency_spike: u32,
    gossipsub: Gossipsub,
}

impl Chaos {
    #[inline]
    pub fn new(chaos_budget: f32, gossipsub: Gossipsub) -> Result<Self, InfernoError> {
        if chaos_budget < 0.0 || chaos_budget > 1.0 {
            return Err(InfernoError::ParseError("Chaos budget must be between 0 and 1".into()));
        }
        let chaos = Self {
            chaos_budget,
            last_chaos: 0,
            partition_rate: 0.1, // 10% Erfolgsverlust als Ziel
            latency_spike: 50,   // 50ms Latenzspitzen als Ziel
            gossipsub,
        };
        info!("Initialized Chaos with budget={}, partition_rate={}, latency_spike={}ms", chaos_budget, chaos.partition_rate, chaos.latency_spike);
        Ok(chaos)
    }

    #[inline]
    pub async fn induce_partition(&self, shards: &mut Vec<Shard>) -> Result<(), InfernoError> {
        self.check_rate_limit(1)?;
        let affected_shards = (shards.len() as f32 * self.chaos_budget) as usize;
        for shard in shards.iter_mut().take(affected_shards) {
            let topic = IdentTopic::new(format!("insc_chaos_{}", shard.id));
            let chaos_msg = format!("partition:rate={:.2},latency={}", self.partition_rate, self.latency_spike).into_bytes();
            self.gossipsub.publish(topic.clone(), chaos_msg)?;

            // Warte auf echte Netzwerkreaktion (z. B. Latenzänderungen von Nachbarn)
            sleep(Duration::from_millis(self.latency_spike as u64)).await;

            // Berechne tatsächliche Auswirkungen basierend auf Netzwerkfeedback
            let avg_latency = shard.neurons.iter().map(|n| n.flamekeeper.latency as f32).sum::<f32>() / shard.neurons.len() as f32;
            shard.success_rate = (shard.success_rate - self.partition_rate).max(0.0);
            for neuron in &mut shard.neurons {
                neuron.flamekeeper.latency = (neuron.flamekeeper.latency + self.latency_spike).min(1000); // Begrenze auf 1s
            }
            debug!("Induced partition on shard {}: success_rate={:.2}, avg_latency={:.2}ms", shard.id, shard.success_rate, avg_latency);
        }
        info!("Induced partition on {} of {} shards with chaos_budget={}", affected_shards, shards.len(), self.chaos_budget);
        Ok(())
    }

    #[inline]
    pub fn adjust_chaos_budget(&mut self, system_load: f32) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?;
        self.chaos_budget = 0.05_f32.max(0.2_f32.min(0.2 * (1.0 - system_load)));
        debug!("Adjusted chaos budget to {}", self.chaos_budget);
        Ok(())
    }

    fn check_rate_limit(&self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_chaos {
            self.last_chaos = now;
        } else if max_per_sec == 0 {
            return Err(InfernoError::Network("Chaos rate limit exceeded".into()));
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
    async fn test_chaos_induction() {
        let keypair = Keypair::generate_ed25519();
        let gossipsub = Gossipsub::new(keypair.into(), GossipsubConfigBuilder::default().build().unwrap()).unwrap();
        let mut chaos = Chaos::new(0.1, gossipsub).unwrap();
        let mut shard = Shard::new(1, "EU").unwrap();
        shard.add_neuron(Neuron::new(1, Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap()).unwrap()).unwrap();
        let mut shards = vec![shard];
        let initial_success = shards[0].success_rate;
        let initial_latency = shards[0].neurons[0].flamekeeper.latency;
        assert!(chaos.induce_partition(&mut shards).await.is_ok());
        assert!(shards[0].success_rate < initial_success);
        assert!(shards[0].neurons[0].flamekeeper.latency > initial_latency);
    }
}