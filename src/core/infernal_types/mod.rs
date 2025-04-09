// src/core/infernal_types/mod.rs
pub mod infernal_types_flamekeeper;
pub mod infernal_types_flamecall;
pub mod infernal_types_emberblock;
pub mod infernal_keys; // Korrigierter Modulname

pub use infernal_types_flamecall::Flamecall;
pub use infernal_types_flamekeeper::Flamekeeper;
pub use infernal_types_emberblock::Emberblock;
pub use infernal_keys::InfernalKeys; // Korrigierter Re-Export

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types_serialization() {
        let validator = Flamekeeper::new(1, 500, 0.0, 0.0, 6.0, 4.0, 50.0).unwrap(); // 7 Argumente, unwrap hinzugefügt
        assert_eq!(validator.id, 1);
    }
}