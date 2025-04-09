// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_logic.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::{Shard, Neuron, SuperNeuron, INSCFlamecall, INSCEmberblock};
use crate::core::infernal_types::infernal_types_flamecall::Flamecall;
use crate::core::infernal_types::infernal_types_flamekeeper::Flamekeeper;
use crate::core::infernal_types::infernal_keys::InfernalKeys;
use crate::core::error::InfernoError;
use log::{debug, info, warn};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use tokio;
use libp2p::gossipsub::{Gossipsub, GossipsubConfigBuilder};
use libp2p::identity::Keypair;

#[derive(Debug)]
pub struct InfernalNeuroSwarmConsensus {
    lilith: super::infernal_neuroswarm_consensus_lilith::Lilith,
    shardforge: super::infernal_neuroswarm_consensus_shardforge::Shardforge,
    microflame: super::infernal_neuroswarm_consensus_microflame::Microflame,
    swarmforge: super::infernal_neuroswarm_consensus_swarmforge::Swarmforge,
    finalflame: super::infernal_neuroswarm_consensus_finalflame::Finalflame,
    vrf: super::infernal_neuroswarm_consensus_vrf::VRF,
    adc: super::infernal_neuroswarm_consensus_adc::ADC,
    bloom: super::infernal_neuroswarm_consensus_bloom::BloomScheduler,
    geofire: super::infernal_neuroswarm_consensus_geofire::Geofire,
    storage: super::infernal_neuroswarm_consensus_storage::ConsensusStorage,
    flamehash: super::infernal_neuroswarm_consensus_flamehash::Flamehash,
    chaos: super::infernal_neuroswarm_consensus_chaos::Chaos,
    keys: InfernalKeys,
    last_process: u64,
    validators: Vec<Flamekeeper>,
    mempool: Vec<INSCFlamecall>,
    block_height: u64,
    shard_map: HashMap<u64, Shard>,
    gossipsub: Gossipsub,
}

impl InfernalNeuroSwarmConsensus {
    #[inline]
    pub fn new(
        lilith: super::infernal_neuroswarm_consensus_lilith::Lilith,
        shardforge: super::infernal_neuroswarm_consensus_shardforge::Shardforge,
        microflame: super::infernal_neuroswarm_consensus_microflame::Microflame,
        swarmforge: super::infernal_neuroswarm_consensus_swarmforge::Swarmforge,
        finalflame: super::infernal_neuroswarm_consensus_finalflame::Finalflame,
        vrf: super::infernal_neuroswarm_consensus_vrf::VRF,
        adc: super::infernal_neuroswarm_consensus_adc::ADC,
        bloom: super::infernal_neuroswarm_consensus_bloom::BloomScheduler,
        geofire: super::infernal_neuroswarm_consensus_geofire::Geofire,
        storage: super::infernal_neuroswarm_consensus_storage::ConsensusStorage,
        flamehash: super::infernal_neuroswarm_consensus_flamehash::Flamehash,
        chaos: super::infernal_neuroswarm_consensus_chaos::Chaos,
        keys: InfernalKeys,
        validators: Vec<Flamekeeper>,
    ) -> Result<Self, InfernoError> {
        let keypair = Keypair::generate_ed25519();
        let gossipsub = Gossipsub::new(keypair.into(), GossipsubConfigBuilder::default().build().unwrap())?;
        let insc = Self {
            lilith,
            shardforge,
            microflame,
            swarmforge,
            finalflame,
            vrf,
            adc,
            bloom,
            geofire,
            storage,
            flamehash,
            chaos,
            keys,
            last_process: 0,
            validators,
            mempool: Vec::new(),
            block_height: 0,
            shard_map: HashMap::new(),
            gossipsub,
        };
        info!("Initialized InfernalNeuroSwarmConsensus with {} validators", insc.validators.len());
        Ok(insc)
    }

    #[inline]
    pub async fn process_batch(&mut self, batch: Vec<INSCFlamecall>) -> Result<INSCEmberblock, InfernoError> {
        self.check_rate_limit(10)?;

        // Schritt 1: Mempool aktualisieren und Spam filtern
        self.mempool.extend(batch);
        let mut filtered_batch = self.lilith.filter_spam(self.mempool.clone()).await?;
        let mut to_remove = Vec::new();
        for tx in &filtered_batch {
            if !self.bloom.schedule_aggregation(tx, true).await? {
                to_remove.push(tx.core.id);
                debug!("Transaction {} marked for removal by Bloom filter", tx.core.id);
            }
        }
        filtered_batch.retain(|tx| !to_remove.contains(&tx.core.id));
        if filtered_batch.is_empty() {
            return Err(InfernoError::Network("No valid transactions after filtering".into()));
        }
        debug!("Filtered batch: {} transactions remaining after Bloom", filtered_batch.len());

        // Schritt 2: Chaos Engineering und Selbstheilung
        let system_load = self.validators.iter().map(|v| v.activity).sum::<f32>() / self.validators.len() as f32;
        self.chaos.adjust_chaos_budget(system_load)?;
        let mut shards = self.storage.shards.values().cloned().collect::<Vec<Shard>>();
        self.chaos.induce_partition(&mut shards).await?;
        self.lilith.update_shards(shards.clone());
        for shard in &shards {
            self.shard_map.insert(shard.id, shard.clone());
            self.storage.store_shard(shard.clone())?;
        }
        self.self_heal().await?;
        debug!("Chaos induced on {} shards, self-healing completed", shards.len());

        // Schritt 3: Shard-Zuweisung und Mikrokonsens
        self.shardforge.assign_validators_to_shards(&self.validators, filtered_batch.len() as f32 / 0.1)?;
        let mut assigned_batch = Vec::new();
        for tx in &filtered_batch {
            self.microflame.assign_task(tx.clone()).await?;
            assigned_batch.push(tx.clone());
        }
        self.shard_map.insert(self.microflame.shard.id, self.microflame.shard.clone());
        self.storage.store_shard(self.microflame.shard.clone())?;
        debug!("Assigned {} transactions to shards", assigned_batch.len());

        // Schritt 4: Latenzoptimierung und Heat Mapping
        self.geofire.optimize_latency(&self.validators)?;
        debug!("Latency optimized for {} validators", self.validators.len());

        // Schritt 5: Duty-Cycling und Shard-Anpassung
        let current_tps = assigned_batch.len() as f32 / 0.1;
        let avg_latency = self.validators.iter().map(|v| v.latency as f32).sum::<f32>() / self.validators.len() as f32;
        self.adc.update_parameters(avg_latency, current_tps, self.validators.len())?;
        self.adc.adjust_shard_thresholds()?;
        self.adc.apply_duty_cycling(current_tps * 2.0)?;
        debug!("Duty-cycling applied: TPS={:.2}, avg_latency={:.2}ms", current_tps, avg_latency);

        // Schritt 6: Aggregation
        let confirmed = self.swarmforge.aggregate_batch(assigned_batch, &self.keys)?;
        debug!("Aggregated {} transactions", confirmed.len());

        // Schritt 7: VRF für Super-Neuron-Auswahl
        let selected = self.vrf.select_super_neurons(&confirmed, 0.5, 0.1, avg_latency)?;
        let super_neuron_id = selected.first().copied().ok_or(InfernoError::Network("No super neuron selected after VRF".into()))?;
        debug!("Selected super neuron: {}", super_neuron_id);

        // Schritt 8: Block erstellen und hashen
        let core_tx: Vec<Flamecall> = confirmed.into_iter().map(|tx| {
            let hash = self.flamehash.hash_transaction(&tx).unwrap();
            let mut core = tx.core;
            core.data = Some(hash.into_bytes());
            core
        }).collect();
        let shard_ids = self.shard_map.keys().copied().collect();
        let previous_hash = if self.block_height == 0 {
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string()
        } else {
            let shard = self.storage.retrieve_shard(self.block_height)?;
            self.flamehash.hash_block(&INSCEmberblock::from(&shard))?
        };
        let mut block = INSCEmberblock::new(
            super::super::infernal_types::infernal_types_emberblock::Emberblock::new(
                self.block_height + 1,
                self.microflame.shard.id, // Verwende echte shard_id
                &previous_hash,
                core_tx,
                Some(&self.keys),
            )?,
            shard_ids,
            super_neuron_id,
        )?;
        debug!("Created block {} with {} transactions", block.core.height, block.core.transactions.len());

        // Schritt 9: Finalität
        self.finalflame.finalize_global(&mut block, &self.keys)?;
        debug!("Block {} finalized", block.core.height);

        // Schritt 10: MEV-Verteilung und Speicherung
        let total_mev: f32 = block.core.transactions.iter().map(|tx| tx.mev_value).sum();
        self.distribute_mev(total_mev)?;
        let block_hash = self.flamehash.hash_block(&block)?;
        block.core.data = Some(block_hash.into_bytes());
        self.storage.store_shard(self.microflame.shard.clone())?;
        self.block_height += 1;
        self.mempool.clear();

        info!("Processed batch into block {} with hash {}", block.core.height, block_hash);
        Ok(block)
    }

    #[inline]
    fn distribute_mev(&mut self, total_mev: f32) -> Result<(), InfernoError> {
        let cashback = total_mev * 0.05;
        let validators_reward = total_mev * 0.60;
        let burn = total_mev * 0.20;
        let treasury = total_mev * 0.05;
        let dev_pool = total_mev * 0.10;

        let total_capacity: f32 = self.validators.iter().map(|v| v.capacity_score).sum();
        const MAX_MEV: f32 = 1000.0; // Definierte maximale MEV-Grenze
        for validator in &mut self.validators {
            let contribution = validator.capacity_score / total_capacity;
            let reward = validators_reward * contribution * (total_mev / MAX_MEV).min(1.0); // Skalierung basierend auf max. MEV
            validator.reward += reward;
            debug!("Validator {} received MEV reward: {:.2}", validator.id, reward);
        }

        debug!("MEV Distribution: cashback={:.2}, validators={:.2}, burn={:.2}, treasury={:.2}, dev_pool={:.2}", 
               cashback, validators_reward, burn, treasury, dev_pool);
        Ok(())
    }

    fn check_rate_limit(&mut self, max_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if now != self.last_process {
            self.last_process = now;
        } else if max_per_sec == 0 {
            return Err(InfernoError::Network("INSC rate limit exceeded".into()));
        }
        Ok(())
    }

    pub fn get_validators(&self) -> &[Flamekeeper] {
        &self.validators
    }

    pub fn get_shards(&self) -> Vec<&Shard> {
        self.storage.shards.values().collect()
    }

    pub fn update_validators(&mut self, validators: Vec<Flamekeeper>) -> Result<(), InfernoError> {
        self.validators = validators;
        info!("Updated validators: new count={}", self.validators.len());
        Ok(())
    }

    pub async fn self_heal(&mut self) -> Result<(), InfernoError> {
        self.shardforge.self_heal().await?;
        debug!("Self-healing completed for {} shards", self.storage.shards.len());
        Ok(())
    }
}

impl From<&Shard> for INSCEmberblock {
    fn from(shard: &Shard) -> Self {
        let super_neuron_id = shard.neurons.iter()
            .max_by(|a, b| a.capacity_score.partial_cmp(&b.capacity_score).unwrap_or(std::cmp::Ordering::Equal))
            .map(|n| n.id)
            .unwrap_or(1); // Fallback auf 1, falls keine Neuronen vorhanden
        INSCEmberblock {
            core: super::super::infernal_types::infernal_types_emberblock::Emberblock {
                height: 0, // Wird im Konstruktor gesetzt, hier nur Platzhalter für die Struktur
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                transactions: Vec::new(),
                shard_id: shard.id,
                previous_hash: "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string(),
                approval_rate: shard.success_rate,
                data: None,
                signature: None,
            },
            shard_ids: vec![shard.id],
            super_neuron_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use libp2p::gossipsub::{GossipsubConfigBuilder, Gossipsub};
    use libp2p::identity::Keypair;

    #[tokio::test]
    async fn test_insc_full_process() {
        let keypair = Keypair::generate_ed25519();
        let gossipsub = Gossipsub::new(keypair.into(), GossipsubConfigBuilder::default().build().unwrap()).unwrap();
        let fk = Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap();
        let mut shard = Shard::new(1, "EU").unwrap();
        shard.add_neuron(Neuron::new(1, fk.clone()).unwrap()).unwrap();
        let keys = InfernalKeys::new().unwrap();
        let lilith = super::super::infernal_neuroswarm_consensus_lilith::Lilith::new(false, 100, gossipsub.clone(), vec![shard.clone()]).unwrap();
        let shardforge = super::super::infernal_neuroswarm_consensus_shardforge::Shardforge::new(1, 10, 0.1).unwrap();
        let microflame = super::super::infernal_neuroswarm_consensus_microflame::Microflame::new(shard.clone(), 50.0, gossipsub.clone()).unwrap();
        let swarmforge = super::super::infernal_neuroswarm_consensus_swarmforge::Swarmforge::new().unwrap();
        let super_neuron = SuperNeuron::new(1, Neuron::new(1, fk.clone()).unwrap(), Some(&keys)).unwrap();
        let finalflame = super::super::infernal_neuroswarm_consensus_finalflame::Finalflame::new(vec![super_neuron], 0.5, 10).unwrap();
        let vrf = super::super::infernal_neuroswarm_consensus_vrf::VRF::new([0u8; 32]).unwrap();
        let adc = super::super::infernal_neuroswarm_consensus_adc::ADC::new(vec![shard.clone()], 50.0, 1000.0, 1).unwrap();
        let bloom = super::super::infernal_neuroswarm_consensus_bloom::BloomScheduler::new(1000, 0.01).unwrap();
        let geofire = super::super::infernal_neuroswarm_consensus_geofire::Geofire::new(vec![shard.clone()]).unwrap();
        let mut storage = super::super::infernal_neuroswarm_consensus_storage::ConsensusStorage::new().unwrap();
        storage.store_shard(shard.clone()).unwrap();
        let flamehash = super::super::infernal_neuroswarm_consensus_flamehash::Flamehash::new();
        let chaos = super::super::infernal_neuroswarm_consensus_chaos::Chaos::new(0.1, gossipsub.clone()).unwrap();

        let mut insc = InfernalNeuroSwarmConsensus::new(
            lilith, shardforge, microflame, swarmforge, finalflame, vrf, adc, bloom, geofire, storage, flamehash, chaos, keys, vec![fk]
        ).unwrap();

        let core_tx = Flamecall::new(1, "Alice", "Bob", 100, 100_000, 0, None, None).unwrap();
        let tx = INSCFlamecall::new(core_tx, 1.0).unwrap();
        let batch = vec![tx];
        let block = insc.process_batch(batch).await.unwrap();
        assert_eq!(block.core.height, 1);
        assert!(block.core.signature.is_some());
        assert_eq!(insc.storage.shards.len(), 2);
    }

    #[tokio::test]
    async fn test_stress_high_tps() {
        let keypair = Keypair::generate_ed25519();
        let gossipsub = Gossipsub::new(keypair.into(), GossipsubConfigBuilder::default().build().unwrap()).unwrap();
        let fk = Flamekeeper::new(1, 500, 50.0, 10.0, 6.0, 4.0, 50.0).unwrap();
        let mut shard = Shard::new(1, "EU").unwrap();
        shard.add_neuron(Neuron::new(1, fk.clone()).unwrap()).unwrap();
        let keys = InfernalKeys::new().unwrap();
        let lilith = super::super::infernal_neuroswarm_consensus_lilith::Lilith::new(false, 100, gossipsub.clone(), vec![shard.clone()]).unwrap();
        let shardforge = super::super::infernal_neuroswarm_consensus_shardforge::Shardforge::new(1, 10, 0.1).unwrap();
        let microflame = super::super::infernal_neuroswarm_consensus_microflame::Microflame::new(shard.clone(), 50.0, gossipsub.clone()).unwrap();
        let swarmforge = super::super::infernal_neuroswarm_consensus_swarmforge::Swarmforge::new().unwrap();
        let super_neuron = SuperNeuron::new(1, Neuron::new(1, fk.clone()).unwrap(), Some(&keys)).unwrap();
        let finalflame = super::super::infernal_neuroswarm_consensus_finalflame::Finalflame::new(vec![super_neuron], 0.5, 10).unwrap();
        let vrf = super::super::infernal_neuroswarm_consensus_vrf::VRF::new([0u8; 32]).unwrap();
        let adc = super::super::infernal_neuroswarm_consensus_adc::ADC::new(vec![shard.clone()], 50.0, 1000.0, 1).unwrap();
        let bloom = super::super::infernal_neuroswarm_consensus_bloom::BloomScheduler::new(1000, 0.01).unwrap();
        let geofire = super::super::infernal_neuroswarm_consensus_geofire::Geofire::new(vec![shard.clone()]).unwrap();
        let mut storage = super::super::infernal_neuroswarm_consensus_storage::ConsensusStorage::new().unwrap();
        storage.store_shard(shard.clone()).unwrap();
        let flamehash = super::super::infernal_neuroswarm_consensus_flamehash::Flamehash::new();
        let chaos = super::super::infernal_neuroswarm_consensus_chaos::Chaos::new(0.1, gossipsub.clone()).unwrap();

        let mut insc = InfernalNeuroSwarmConsensus::new(
            lilith, shardforge, microflame, swarmforge, finalflame, vrf, adc, bloom, geofire, storage, flamehash, chaos, keys, vec![fk]
        ).unwrap();

        let mut batch = Vec::with_capacity(560000);
        for i in 1..=560000 {
            let core_tx = Flamecall::new(i, "Alice", "Bob", 100, 100_000, 0, None, None).unwrap();
            batch.push(INSCFlamecall::new(core_tx, 1.0).unwrap());
        }
        let start = Instant::now();
        let block = insc.process_batch(batch).await.unwrap();
        let duration = start.elapsed().as_secs_f32();
        assert!(duration < 0.095, "Finality exceeded: {:.3}s", duration);
        assert_eq!(block.core.height, 1);
    }
}