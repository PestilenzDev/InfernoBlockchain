// src/core/infernal_runtime/mod.rs

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub mod infernal_runtime_execution;
// pub mod infernal_runtime_flamehooks; // Auskommentiert, bis wir es implementieren

pub use infernal_runtime_execution::*;
// pub use infernal_runtime_flamehooks::*; // Auskommentiert, bis wir es implementieren

/// Zustand der Laufzeitumgebung.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InfernalRuntime {
    pub current_block_height: u64,
    pub balances: HashMap<String, u64>, // Zustandsspeicher für Salden
}

impl InfernalRuntime {
    pub fn new() -> Self {
        InfernalRuntime {
            current_block_height: 0,
            balances: HashMap::new(),
        }
    }

    pub fn increment_block(&mut self) {
        self.current_block_height += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_init() {
        let mut runtime = InfernalRuntime::new();
        assert_eq!(runtime.current_block_height, 0);
        assert!(runtime.balances.is_empty());
        runtime.increment_block();
        assert_eq!(runtime.current_block_height, 1);
    }
}