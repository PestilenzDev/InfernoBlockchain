// src/lib.rs
use std::result::Result;

pub mod core;

pub use core::{
    initialize_core,
    Flamecall as CoreFlamecall,
    Flamekeeper,
    Emberblock as CoreEmberblock,
    InfernalKeys,
};

pub use core::infernal_neuroswarm_consensus::{
    Shard,
    Neuron,
    SuperNeuron,
    Flamecall,
    Emberblock,
};

pub fn start_inferno() -> Result<(), String> {
    initialize_core()?;
    log::info!("Inferno Blockchain started with Infernal NeuroSwarm Consensus");
    Ok(())
}

pub fn placeholder() {
    println!("Module under construction");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_start() {
        let _ = env_logger::builder().is_test(true).try_init();
        assert!(start_inferno().is_ok(), "Failed to start Inferno library");
    }

    #[tokio::test]
    #[ignore]
    async fn test_consensus_init() {
        // Wird später mit INSC angepasst
    }

    #[tokio::test]
    #[ignore]
    async fn test_full_consensus_flow() {
        // Wird später mit INSC angepasst
    }
}