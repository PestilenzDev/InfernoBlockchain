// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_vrf.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::INSCFlamecall;
use crate::core::error::InfernoError;
use log::{debug, info};
use schnorrkel::{Keypair, PublicKey, Signature, vrf::{VRFInOut, VRFProof, VRFPreOut}};
use rand::Rng;
use tch::{nn, Tensor, Device, Kind};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct VRF {
    nn: nn::Sequential,
    base_seed: [u8; 32],
    keypair: Keypair,
}

impl VRF {
    #[inline]
    pub fn new(base_seed: [u8; 32]) -> Result<Self, InfernoError> {
        let vs = nn::VarStore::new(Device::Cpu);
        let nn = nn::seq()
            .add(nn::linear(&vs.root(), 3, 8, Default::default()))
            .add_fn(|xs| xs.relu())
            .add(nn::linear(&vs.root(), 8, 1, Default::default()));
        let keypair = Keypair::generate();
        let vrf = Self { nn, base_seed, keypair };
        info!("Initialized VRF with base_seed={}", hex::encode(&base_seed[..8]));
        Ok(vrf)
    }

    #[inline]
    pub fn generate_seed(&self, mev_potential: f32, stake_volatility: f32, latency_forecast: f32) -> Result<[u8; 32], InfernoError> {
        let inputs = Tensor::of_slice(&[mev_potential, stake_volatility, latency_forecast])
            .view([1, 3])
            .to_kind(Kind::Float);
        let nn_output = self.nn.forward(&inputs).double_value(&[0]) as f32;
        let mut seed = self.base_seed;
        let adjustment = (nn_output * 1_000_000.0) as u64;
        for (i, byte) in adjustment.to_le_bytes().iter().enumerate() {
            seed[i] ^= *byte;
        }
        debug!("Generated VRF seed with mev={:.2}, stake_vol={:.2}, latency={:.2}, output={}", mev_potential, stake_volatility, latency_forecast, hex::encode(&seed[..8]));
        Ok(seed)
    }

    #[inline]
    pub fn select_super_neurons(&self, validators: &[INSCFlamecall], mev_potential: f32, stake_volatility: f32, latency_forecast: f32) -> Result<Vec<u64>, InfernoError> {
        let seed = self.generate_seed(mev_potential, stake_volatility, latency_forecast)?;
        let mut selected = Vec::new();

        for tx in validators {
            let mut extra = [0u8; 32];
            extra[..8].copy_from_slice(&tx.core.id.to_le_bytes());
            let (io, proof) = self.keypair.vrf_sign(VRFInOut::new(&seed, &extra))?;
            let public_key = self.keypair.public;
            if io.verify(&proof, &public_key).is_ok() {
                let score = io.to_preout().to_bytes()[0] as f32 / 255.0; // Normalisiere auf [0,1]
                if score < 0.1 { // 10% der besten Scores (anpassbar)
                    selected.push(tx.core.id);
                    debug!("Selected super neuron {} with VRF score={:.2}", tx.core.id, score);
                }
            }
        }

        info!("Selected {} super neurons from {} validators", selected.len(), validators.len());
        Ok(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vrf_seed_generation() {
        let base_seed = [0u8; 32];
        let vrf = VRF::new(base_seed).unwrap();
        let seed1 = vrf.generate_seed(0.5, 0.1, 10.0).unwrap();
        let seed2 = vrf.generate_seed(0.5, 0.1, 10.0).unwrap();
        assert_eq!(seed1, seed2); // Deterministisch für gleiche Eingaben
        let seed3 = vrf.generate_seed(0.6, 0.1, 10.0).unwrap();
        assert_ne!(seed1, seed3); // Unterschiedlich für andere Eingaben
    }

    #[test]
    fn test_vrf_super_neuron_selection() {
        let base_seed = [0u8; 32];
        let vrf = VRF::new(base_seed).unwrap();
        let mut validators = Vec::new();
        for i in 1..=100 {
            let core_tx = CoreFlamecall::new(i, "Alice", "Bob", 100, 100_000, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), None, None).unwrap();
            validators.push(INSCFlamecall::new(core_tx, 1.0).unwrap());
        }
        let selected = vrf.select_super_neurons(&validators, 0.5, 0.1, 10.0).unwrap();
        assert!(!selected.is_empty());
        assert!(selected.len() <= validators.len());
        assert!(selected.len() > 0); // Sollte mindestens einige auswählen
        let selected_again = vrf.select_super_neurons(&validators, 0.5, 0.1, 10.0).unwrap();
        assert_eq!(selected, selected_again); // Deterministisch für gleiche Eingaben
    }
}