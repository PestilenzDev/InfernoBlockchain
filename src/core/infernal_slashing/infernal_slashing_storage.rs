// src/core/infernal_slashing/infernal_slashing_storage.rs
use crate::core::error::InfernoError;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)] // Clone für SlashingStorage hinzufügen
pub struct SlashingStorage {
    offenses: HashMap<String, u32>,
    history: Vec<SlashingEvent>,
}

#[derive(Debug, Clone)]
pub struct SlashingEvent {
    pub validator_id: String,
    pub offense_count: u32,
    pub slash_percentage: u32,
    pub timestamp: u64,
}

impl SlashingStorage {
    pub fn new() -> Self {
        SlashingStorage {
            offenses: HashMap::new(),
            history: Vec::new(),
        }
    }

    pub fn increment_offense(&mut self, validator_id: &str) -> u32 {
        let count = self.offenses.entry(validator_id.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    pub fn record_slashing(&mut self, validator_id: &str, offense_count: u32, slash_percentage: u32) -> Result<(), InfernoError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| InfernoError::Network(e.to_string()))?
            .as_secs();
        self.history.push(SlashingEvent {
            validator_id: validator_id.to_string(),
            offense_count,
            slash_percentage,
            timestamp,
        });
        Ok(())
    }

    pub fn get_offense_count(&self, validator_id: &str) -> Option<u32> {
        self.offenses.get(validator_id).copied()
    }

    pub fn get_slashing_history(&self, validator_id: &str) -> Vec<&SlashingEvent> {
        self.history.iter().filter(|event| event.validator_id == validator_id).collect()
    }
}