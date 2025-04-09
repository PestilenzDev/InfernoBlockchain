// src/core/mod.rs
pub mod infernal_config;
pub mod infernal_types;
pub mod infernal_runtime;
pub mod infernal_neuroswarm_consensus; // Ersetzt infernoswarm_consensus
pub mod infernal_slashing;
pub mod infernal_corevault;
pub mod error;
pub mod testutils;
pub mod infernal_mev_flames;

pub use infernal_config::{InfernalConfig, ChainConfig, EmberConfig};
pub use infernal_types::{Flamecall, Flamekeeper, Emberblock, InfernalKeys};
pub use infernal_runtime::{InfernalRuntime, execute_transaction};
pub use infernal_neuroswarm_consensus::{Shard, Neuron, SuperNeuron}; // Neue Typen von INSC
pub use infernal_slashing::{trigger_slashing, SlashingStorage};
pub use infernal_corevault::{Emberchain, Ashpool};
pub use error::InfernoError;
pub use infernal_mev_flames::{InfernalMevFlames, MevMode};

pub fn initialize_core() -> Result<(), String> {
    let config = InfernalConfig::default();
    log::info!("Inferno Core initialized with config: {:?}", config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_initialization() {
        let _ = env_logger::builder().is_test(true).try_init();
        assert!(initialize_core().is_ok(), "Core initialization failed");
    }

    #[test]
    fn test_config_creation() {
        let _ = env_logger::builder().is_test(true).try_init();
        let config = InfernalConfig::default();
        assert_eq!(config.chain_config.chain_id, 1327);
        assert_eq!(config.ember_config.gas_base_fee, 0.00001);
    }
}