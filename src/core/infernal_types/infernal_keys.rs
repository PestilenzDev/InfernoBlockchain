// src/core/infernal_types/infernal_types_infernal_keys.rs

use serde::{Serialize, Deserialize};
use schnorrkel::{Keypair, PublicKey, Signature, keys::MiniSecretKey, ExpansionMode};
use rand::rngs::OsRng;
use crate::core::error::InfernoError;
use log::info;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InfernalKeys {
    pub keypair: Keypair,
}

impl InfernalKeys {
    #[inline]
    pub fn new() -> Result<Self, InfernoError> {
        let mut csprng = OsRng{};
        let keypair = Keypair::generate_with(&mut csprng);
        let keys = Self { keypair };
        info!("Generated new InfernalKeys: pubkey={}", hex::encode(keys.public_key().to_bytes()));
        Ok(keys)
    }

    #[inline]
    pub fn from_seed(seed: &[u8]) -> Result<Self, InfernoError> {
        let mini_secret = MiniSecretKey::from_bytes(seed)
            .map_err(|e| InfernoError::ParseError(e.to_string()))?;
        let keypair = mini_secret.expand_to_keypair(ExpansionMode::Uniform);
        let keys = Self { keypair };
        info!("Created InfernalKeys from seed: pubkey={}", hex::encode(keys.public_key().to_bytes()));
        Ok(keys)
    }

    #[inline]
    pub fn sign(&self, message: &[u8]) -> Result<Signature, InfernoError> {
        Ok(self.keypair.sign_simple(b"inferno", message))
    }

    #[inline]
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool, InfernoError> {
        self.keypair.public.verify_simple(b"inferno", message, signature)
            .map_err(|e| InfernoError::NoiseError(e.to_string()))?;
        Ok(true)
    }

    #[inline]
    pub fn public_key(&self) -> PublicKey {
        self.keypair.public
    }

    pub fn rotate_keys(&mut self, new_seed: &[u8]) -> Result<(), InfernoError> {
        let mini_secret = MiniSecretKey::from_bytes(new_seed)
            .map_err(|e| InfernoError::ParseError(e.to_string()))?;
        self.keypair = mini_secret.expand_to_keypair(ExpansionMode::Uniform);
        info!("Rotated keys for InfernalKeys: new pubkey={}", hex::encode(self.public_key().to_bytes()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_rotation() {
        let mut keys = InfernalKeys::new().unwrap();
        let old_pubkey = keys.public_key();
        let new_seed = [1u8; 32];
        keys.rotate_keys(&new_seed).unwrap();
        assert_ne!(old_pubkey.to_bytes(), keys.public_key().to_bytes());
        let message = b"Test rotation";
        let sig = keys.sign(message).unwrap();
        assert!(keys.verify(message, &sig).unwrap());
    }
}