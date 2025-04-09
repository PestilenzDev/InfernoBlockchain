// src/core/infernal_mev_flames/infernal_mev_flames_types.rs
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum MevMode {
    Basic,   // Ember Flow + Ash Tide
    Surge,   // Flame Pulse + Blaze Circuit
    Scale,   // Pyre Shard + Ember Flow
    Living,  // Alle 5 Systeme
}

#[derive(Debug, Clone)]
pub struct MevMetrics {
    pub tps: f64,
    pub mev_potential: f64, // INF/Min
    pub active_wallets: usize,
    pub validator_latency: f64, // ms
}

#[derive(Debug, Clone)]
pub struct FlamecoreNexus {
    pub mode: MevMode,
    pub metrics: MevMetrics,
    pub last_update: u64,
    pub heat_index: HashMap<String, f64>, // Shard -> Heat Index (0-100)
    pub dao_override: Option<MevMode>,    // DAO-Override
}

#[derive(Debug)]
pub struct InfernalMevFlames {
    pub nexus: Arc<Mutex<FlamecoreNexus>>,
    pub system_pools: HashMap<String, f64>, // z.B. "FlamePulse" -> INF
}