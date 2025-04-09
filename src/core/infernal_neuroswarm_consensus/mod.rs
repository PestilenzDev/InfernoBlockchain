// src/core/infernal_neuroswarm_consensus/mod.rs
/// Infernal NeuroSwarm Consensus (INSC) - A hybrid consensus mechanism combining swarm intelligence
/// and neural networks, controlled by Lilith's Infernal Cortex.
pub mod infernal_neuroswarm_consensus_adc;
pub mod infernal_neuroswarm_consensus_bloom;
pub mod infernal_neuroswarm_consensus_chaos;
pub mod infernal_neuroswarm_consensus_finalflame;
pub mod infernal_neuroswarm_consensus_geofire;
pub mod infernal_neuroswarm_consensus_lilith;
pub mod infernal_neuroswarm_consensus_logic;
pub mod infernal_neuroswarm_consensus_microflame;
pub mod infernal_neuroswarm_consensus_shardforge;
pub mod infernal_neuroswarm_consensus_storage;
pub mod infernal_neuroswarm_consensus_swarmforge;
pub mod infernal_neuroswarm_consensus_types;
pub mod infernal_neuroswarm_consensus_vrf;
pub mod infernal_neuroswarm_consensus_flamehash;

// Re-export key types for easier access
pub use infernal_neuroswarm_consensus_types::{Shard, Neuron, SuperNeuron, Flamecall, Emberblock};
pub use infernal_neuroswarm_consensus_lilith::Lilith;
pub use infernal_neuroswarm_consensus_shardforge::Shardforge;
pub use infernal_neuroswarm_consensus_microflame::Microflame;
pub use infernal_neuroswarm_consensus_swarmforge::Swarmforge;
pub use infernal_neuroswarm_consensus_finalflame::Finalflame;
pub use infernal_neuroswarm_consensus_adc::ADC;
pub use infernal_neuroswarm_consensus_vrf::VRF;
pub use infernal_neuroswarm_consensus_chaos::Chaos;
pub use infernal_neuroswarm_consensus_flamehash::Flamehash;
pub use infernal_neuroswarm_consensus_logic::InfernalNeuroSwarmConsensus;
pub use infernal_neuroswarm_consensus_geofire::Geofire;
pub use infernal_neuroswarm_consensus_bloom::BloomScheduler;
pub use infernal_neuroswarm_consensus_storage::ConsensusStorage;