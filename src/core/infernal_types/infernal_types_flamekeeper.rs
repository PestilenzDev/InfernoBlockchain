// src/core/infernal_types/infernal_types_flamekeeper.rs

use serde::{Serialize, Deserialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use log::{debug, info, warn};
use crate::core::error::InfernoError;

/// Repräsentiert einen Validator im Infernal NeuroSwarm Consensus (INSC).
/// Whitepaper: 2.6 Validator-Infrastruktur - Dynamische Kapazitätsbewertung und Hardware-Unterstützung.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Flamekeeper {
    pub id: u64,
    pub stake: u64,
    pub latitude: f64,
    pub longitude: f64,
    pub cpu_units: f64,
    pub ram_gb: f64,
    pub bandwidth_mbps: f64,
    pub capacity_score: f32,
    pub latency: u64,
    pub is_leader_eligible: bool,
    pub activity: f32,
    pub standby: bool,
    /// Zeitstempel des letzten Updates für Rate-Limiting (DDoS-Schutz).
    last_update: u64,
    /// Anzahl der Updates in der aktuellen Sekunde (DDoS-Schutz).
    update_count: u32,
}

impl Flamekeeper {
    #[inline]
    pub fn new(
        id: u64,
        stake: u64,
        latitude: f64,
        longitude: f64,
        cpu_units: f64,
        ram_gb: f64,
        bandwidth_mbps: f64,
    ) -> Result<Self, InfernoError> {
        if stake == 0 {
            return Err(InfernoError::Network("Stake must be greater than 0".into()));
        }
        if cpu_units <= 0.0 || ram_gb <= 0.0 || bandwidth_mbps <= 0.0 {
            return Err(InfernoError::Network("Hardware parameters must be positive".into()));
        }
        if latitude < -90.0 || latitude > 90.0 || longitude < -180.0 || longitude > 180.0 {
            return Err(InfernoError::ParseError("Invalid latitude or longitude".into()));
        }

        let capacity_score = (0.6 * cpu_units + 0.2 * ram_gb + 0.2 * bandwidth_mbps) as f32;
        let fk = Self {
            id,
            stake,
            latitude,
            longitude,
            cpu_units,
            ram_gb,
            bandwidth_mbps,
            capacity_score,
            latency: 0,
            is_leader_eligible: capacity_score >= 50.0,
            activity: 1.0,
            standby: false,
            last_update: 0,
            update_count: 0,
        };
        info!("Created Flamekeeper {}: capacity_score={:.2}, stake={}", id, capacity_score, stake);
        Ok(fk)
    }

    #[inline]
    pub fn update_latency(&mut self, latency: u64) -> Result<(), InfernoError> {
        self.check_rate_limit(10)?; // Max 10 Updates pro Sekunde
        if latency > 10_000 {
            warn!("Flamekeeper {} latency too high: {}ms", self.id, latency);
            return Err(InfernoError::LatencyTooHigh(latency));
        }
        self.latency = latency;
        self.activity = if latency < 50 { 1.0 } else { 0.8 - (latency as f32 / 1000.0).min(0.8) };
        if self.activity < 0.05 {
            self.standby = true;
            debug!("Flamekeeper {} set to standby: activity={:.2}", self.id, self.activity);
        }
        info!("Flamekeeper {} updated: latency={}ms, activity={:.2}", self.id, latency, self.activity);
        Ok(())
    }

    #[inline]
    pub fn recalculate_capacity(&mut self) -> Result<(), InfernoError> {
        if self.cpu_units <= 0.0 || self.ram_gb <= 0.0 || self.bandwidth_mbps <= 0.0 {
            return Err(InfernoError::Network("Hardware parameters must be positive".into()));
        }
        self.capacity_score = (0.6 * self.cpu_units + 0.2 * self.ram_gb + 0.2 * self.bandwidth_mbps) as f32;
        self.is_leader_eligible = self.capacity_score >= 50.0;
        Ok(())
    }

    /// Prüft Rate-Limiting für DDoS-Schutz.
    fn check_rate_limit(&mut self, max_updates_per_sec: u32) -> Result<(), InfernoError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| InfernoError::Network(e.to_string()))?
            .as_secs();
        if now != self.last_update {
            self.last_update = now;
            self.update_count = 1;
        } else {
            self.update_count += 1;
            if self.update_count > max_updates_per_sec {
                warn!("Flamekeeper {} rate limit exceeded: {} updates/sec", self.id, self.update_count);
                return Err(InfernoError::Network("Update rate limit exceeded".into()));
            }
        }
        Ok(())
    }

    pub fn simulate_activity(&mut self) {
        let mut rng = rand::thread_rng();
        self.activity = rng.gen_range(0.0..1.0);
        self.standby = self.activity < 0.05;
        if self.standby {
            debug!("Flamekeeper {} simulated to standby", self.id);
        }
    }
}

impl fmt::Display for Flamekeeper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Flamekeeper {{ id: {}, stake: {}, capacity_score: {:.2}, latency: {}ms, leader_eligible: {}, activity: {:.2}, standby: {} }}",
            self.id, self.stake, self.capacity_score, self.latency, self.is_leader_eligible, self.activity, self.standby
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flamekeeper_rate_limit() {
        let mut fk = Flamekeeper::new(7, 4000, 45.0, 15.0, 4.0, 8.0, 60.0).unwrap();
        for _ in 0..10 {
            assert!(fk.update_latency(30).is_ok());
        }
        assert!(matches!(
            fk.update_latency(30),
            Err(InfernoError::Network(_))
        ));
    }
}