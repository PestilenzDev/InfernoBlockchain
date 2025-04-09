// src/core/infernal_slashing/mod.rs
pub mod infernal_slashing_logic;
pub mod infernal_slashing_detection;
pub mod infernal_slashing_penalty;
pub mod infernal_slashing_flamewhistle;
pub mod infernal_slashing_storage;

pub use infernal_slashing_logic::trigger_slashing;
pub use infernal_slashing_storage::SlashingStorage;