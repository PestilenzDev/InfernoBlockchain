// src/core/infernal_config/mod.rs

use serde::{Serialize, Deserialize};
use std::fmt;

pub mod infernal_config_chain;
pub mod infernal_config_embers;

pub use infernal_config_chain::*;
pub use infernal_config_embers::*;

/// Grundlegende Konfiguration der Inferno Blockchain.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InfernalConfig {
    pub chain_config: ChainConfig,
    pub ember_config: EmberConfig,
}

impl Default for InfernalConfig {
    fn default() -> Self {
        InfernalConfig {
            chain_config: ChainConfig::default(),
            ember_config: EmberConfig::default(),
        }
    }
}

impl fmt::Display for InfernalConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Chain: {:?}, Embers: {:?}", self.chain_config, self.ember_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = InfernalConfig::default();
        assert_eq!(config.chain_config.chain_id, 1327); // Aus Developer Docs
        assert_eq!(config.ember_config.gas_base_fee, 0.00001); // Aus Whitepaper
    }
}