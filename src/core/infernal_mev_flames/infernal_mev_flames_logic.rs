// src/core/infernal_mev_flames/infernal_mev_flames_logic.rs
use super::{InfernalMevFlames, FlamecoreNexus, MevMode};
use crate::core::infernal_types::{Flamekeeper, Flamecall};
// Temporär auskommentiert, bis INSC vollständig ist:
// use crate::core::infernoswarm_consensus::infernoswarm_consensus_swarmforge::{SwarmConsensus, Group};
// use crate::core::infernoswarm_consensus::infernoswarm_consensus_vrf::generate_vrf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

impl InfernalMevFlames {
    pub fn new() -> Self {
        let nexus = FlamecoreNexus {
            mode: MevMode::Basic,
            metrics: super::MevMetrics { tps: 0.0, mev_potential: 0.0, active_wallets: 0, validator_latency: 0.0 },
            last_update: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            heat_index: HashMap::new(),
            dao_override: None,
        };
        InfernalMevFlames {
            nexus: Arc::new(Mutex::new(nexus)),
            system_pools: HashMap::new(),
        }
    }

    pub async fn process_mev(&mut self, group: &mut Vec<Flamecall>, validators: &mut Vec<Flamekeeper>) -> Result<f64, String> {
        let total_mev = group.iter().map(|tx| tx.amount as f64 * 0.01).sum::<f64>();
        
        let (mode, heat) = {
            let nexus = self.nexus.lock().await;
            let heat = nexus.heat_index.get(&validators[0].location.0.to_string()).unwrap_or(&50.0);
            (nexus.dao_override.clone().unwrap_or(nexus.mode.clone()), *heat)
        };

        match mode {
            MevMode::Basic => {
                self.process_ember_flow(group, total_mev, heat).await?;
                self.process_ash_tide(group, total_mev, heat).await
            }
            MevMode::Surge => {
                self.process_flame_pulse(group, total_mev).await?;
                self.process_blaze_circuit(group, total_mev).await
            }
            MevMode::Scale => {
                self.process_pyre_shard(group, total_mev, heat, validators).await?;
                self.process_ember_flow(group, total_mev, heat).await
            }
            MevMode::Living => {
                self.process_flame_pulse(group, total_mev).await?;
                self.process_ember_flow(group, total_mev, heat).await?;
                self.process_blaze_circuit(group, total_mev).await?;
                self.process_pyre_shard(group, total_mev, heat, validators).await?;
                self.process_ash_tide(group, total_mev, heat).await
            }
        }
    }

    async fn process_ember_flow(&mut self, _group: &mut Vec<Flamecall>, total_mev: f64, heat: f64) -> Result<f64, String> {
        let cashback_rate = if heat < 30.0 { 0.06 } else if heat > 70.0 { 0.04 } else { 0.05 };
        let cashback = total_mev * cashback_rate;
        let _treasury = total_mev * 0.05;
        let validators = total_mev * 0.60;
        let burn = total_mev * 0.20;
        let system_pool = total_mev * 0.10;

        self.system_pools.entry("EmberFlow".to_string()).and_modify(|e| *e += system_pool).or_insert(system_pool);
        log::info!("Ember Flow: Cashback={}, Validators={}, Burn={}, SystemPool={}", cashback, validators, burn, system_pool);
        Ok(total_mev)
    }

    async fn process_ash_tide(&mut self, _group: &mut Vec<Flamecall>, total_mev: f64, heat: f64) -> Result<f64, String> {
        let cashback_rate = if heat < 30.0 { 0.06 } else if heat > 70.0 { 0.04 } else { 0.05 };
        let cashback = total_mev * cashback_rate;
        let _treasury = total_mev * 0.05;
        let validators = total_mev * 0.60;
        let burn = total_mev * 0.20;
        let system_pool = total_mev * 0.10;

        self.system_pools.entry("AshTide".to_string()).and_modify(|e| *e += system_pool).or_insert(system_pool);
        log::info!("Ash Tide: Cashback={}, Validators={}, Burn={}, SystemPool={}", cashback, validators, burn, system_pool);
        Ok(total_mev)
    }

    async fn process_flame_pulse(&mut self, group: &mut Vec<Flamecall>, total_mev: f64) -> Result<f64, String> {
        let chaos_phase = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() % 10 < 3;
        if chaos_phase {
            group.sort_by(|a, b| b.amount.cmp(&a.amount));
            log::info!("Flame Pulse: Chaos phase, prioritizing high-value tx");
        } else {
            // Temporär ohne VRF, bis vrf.rs implementiert ist
            group.sort_by(|a, b| a.id.cmp(&b.id));
            log::info!("Flame Pulse: Order phase, simple ID sorting (VRF pending)");
        }
        let cashback = total_mev * 0.05;
        let _treasury = total_mev * 0.05;
        let validators = total_mev * 0.60;
        let burn = total_mev * 0.20;
        let system_pool = total_mev * 0.10;

        self.system_pools.entry("FlamePulse".to_string()).and_modify(|e| *e += system_pool).or_insert(system_pool);
        log::info!("Flame Pulse: Cashback={}, Validators={}, Burn={}, SystemPool={}", cashback, validators, burn, system_pool);
        Ok(total_mev)
    }

    async fn process_blaze_circuit(&mut self, _group: &mut Vec<Flamecall>, total_mev: f64) -> Result<f64, String> {
        let cashback = total_mev * 0.05;
        let _treasury = total_mev * 0.05;
        let validators = total_mev * 0.60;
        let burn = total_mev * 0.20;
        let system_pool = total_mev * 0.10;

        self.system_pools.entry("BlazeCircuit".to_string()).and_modify(|e| *e += system_pool).or_insert(system_pool);
        log::info!("Blaze Circuit: Cashback={}, Validators={}, Burn={}, SystemPool={}", cashback, validators, burn, system_pool);
        Ok(total_mev)
    }

    async fn process_pyre_shard(&mut self, _group: &mut Vec<Flamecall>, total_mev: f64, heat: f64, _validators: &mut Vec<Flamekeeper>) -> Result<f64, String> {
        let cashback_rate = if heat < 30.0 { 0.06 } else if heat > 70.0 { 0.04 } else { 0.05 };
        let cashback = total_mev * cashback_rate;
        let _treasury = total_mev * 0.05;
        let validators = total_mev * 0.60;
        let burn = total_mev * 0.20;
        let system_pool = total_mev * 0.10;

        self.system_pools.entry("PyreShard".to_string()).and_modify(|e| *e += system_pool).or_insert(system_pool);
        log::info!("Pyre Shard: Cashback={}, Validators={}, Burn={}, SystemPool={}", cashback, validators, burn, system_pool);
        Ok(total_mev)
    }
}