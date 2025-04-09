// src/core/infernal_config/infernal_config_embers.rs

use serde::{Serialize, Deserialize};

/// Konfiguration für Gas („Embers“) und Gebühren.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmberConfig {
    pub gas_base_fee: f64, // Basis-Gebühr in INF (z. B. 0,00001 INF)
    pub gas_limit_per_block: u64, // Max. Gas pro Block
}

impl Default for EmberConfig {
    fn default() -> Self {
        EmberConfig {
            gas_base_fee: 0.00001, // ~0,0005-0,005 USD (Whitepaper)
            gas_limit_per_block: 100_000_000, // Platzhalter, anpassbar
        }
    }
}