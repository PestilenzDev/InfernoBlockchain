// src/core/infernal_mev_flames/mod.rs
pub mod infernal_mev_flames_api;
pub mod infernal_mev_flames_logic;
pub mod infernal_mev_flames_nexus;
pub mod infernal_mev_flames_nexus_basic;
pub mod infernal_mev_flames_nexus_surge;
pub mod infernal_mev_flames_nexus_scale;
pub mod infernal_mev_flames_nexus_living;
pub mod infernal_mev_flames_nexus_embermetrics;
pub mod infernal_mev_flames_pulse;
pub mod infernal_mev_flames_emberflow;
pub mod infernal_mev_flames_blazecircuit;
pub mod infernal_mev_flames_pyreshard;
pub mod infernal_mev_flames_ashtide;
pub mod infernal_mev_flames_infernalwatchdogs;
pub mod infernal_mev_flames_emberstore;
pub mod infernal_mev_flames_types;
pub mod infernal_mev_flames_flamehooks;

pub use infernal_mev_flames_types::{InfernalMevFlames, MevMode, MevMetrics, FlamecoreNexus};
pub use infernal_mev_flames_pulse::process_flame_pulse;