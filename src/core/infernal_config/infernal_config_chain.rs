// src/core/infernal_config/infernal_config_chain.rs

use serde::{Serialize, Deserialize};

/// Konfiguration für die Chain-Identität und grundlegende Parameter.
#[derive (Serialize, Deserialize, Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub chain_name: String,
    pub block_time_ms: u64, // Ziel-Blockzeit in Millisekunden
}

impl Default for ChainConfig {
    fn default() -> Self {
        ChainConfig {
            chain_id: 1327, // Aus Developer Docs: Chain-ID 1327
            chain_name: "Inferno".to_string(),
            block_time_ms: 500, // Ziel: <0,5s Finalität (Whitepaper)
        }
    }
}