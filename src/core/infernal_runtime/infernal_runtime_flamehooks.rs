// src/core/infernal_runtime/infernal_runtime_flamehooks.rs

/// Platzhalter für globale Hooks.
pub fn pre_execution_hook(_runtime: &super::InfernalRuntime) {
    log::debug!("Pre-execution hook called");
}