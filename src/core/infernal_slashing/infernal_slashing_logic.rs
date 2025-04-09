// src/core/infernal_slashing/infernal_slashing_logic.rs
use crate::core::error::InfernoError;
use super::infernal_slashing_storage::SlashingStorage;

pub fn trigger_slashing(validator_id: &str, storage: &mut SlashingStorage) -> Result<(), InfernoError> {
    let offense_count = storage.increment_offense(validator_id);
    let slash_percentage = match offense_count {
        1 => 2,
        2 => 5,
        _ => 50,
    };
    storage.record_slashing(validator_id, offense_count, slash_percentage)?;
    println!("Slashing validator {} with {}% for offense {}", validator_id, slash_percentage, offense_count);
    Ok(())
}