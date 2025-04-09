// src/core/infernal_mev_flames/infernal_mev_flames_nexus.rs
use super::{InfernalMevFlames, MevMode};
use crate::core::infernal_types::Flamekeeper;
// Temporär auskommentiert, bis INSC vollständig ist:
// use crate::core::infernoswarm_consensus::infernoswarm_consensus_swarmforge::SwarmConsensus;
use std::time::{SystemTime, UNIX_EPOCH};

impl InfernalMevFlames {
    pub async fn update_metrics(&self, validators: &[Flamekeeper]) {
        let mut nexus = self.nexus.lock().await;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if now - nexus.last_update >= 2 {
            // Temporäre Werte, bis INSC Metrics liefert
            nexus.metrics.tps = 0.0; // Platzhalter
            nexus.metrics.mev_potential = 0.0; // Platzhalter
            nexus.metrics.active_wallets = 0; // Platzhalter
            nexus.metrics.validator_latency = validators.iter().map(|v| v.latency as f64).sum::<f64>() / validators.len() as f64;
            nexus.last_update = now;

            // Geo-Sharding Logik später mit INSC anpassen
            for v in validators {
                let region = v.location.0.to_string();
                let heat = 50.0; // Platzhalter
                nexus.heat_index.insert(region, heat);
            }

            if nexus.dao_override.is_none() {
                nexus.mode = MevMode::Basic; // Platzhalter
            }
            log::info!("Updated MEV mode: {:?}", nexus.mode);
        }
    }

    pub async fn set_dao_override(&self, mode: MevMode) {
        let mut nexus = self.nexus.lock().await;
        nexus.dao_override = Some(mode.clone());
        log::info!("DAO override set to: {:?}", mode);
    }
}