// src/core/infernal_mev_flames/infernal_mev_flames_pulse.rs
use super::InfernalMevFlames;
use crate::core::infernal_types::Flamecall;
// Temporär auskommentiert, bis INSC vollständig ist:
// use crate::core::infernoswarm_consensus::infernoswarm_consensus_swarmforge::Group;
// use crate::core::infernoswarm_consensus::infernoswarm_consensus_vrf::generate_vrf;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn process_flame_pulse(mev: &mut InfernalMevFlames, group: &mut Vec<Flamecall>, total_mev: f64) -> Result<f64, String> {
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

    mev.system_pools.entry("FlamePulse".to_string()).and_modify(|e| *e += system_pool).or_insert(system_pool);
    log::info!("Flame Pulse: Cashback={}, Validators={}, Burn={}, SystemPool={}", cashback, validators, burn, system_pool);
    Ok(total_mev)
}