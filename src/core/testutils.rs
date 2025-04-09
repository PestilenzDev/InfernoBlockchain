// src/core/testutils.rs
use crate::core::Flamekeeper;
use crate::core::Flamecall;
use rand::{Rng, thread_rng};

pub fn assert_only_eligible_leaders_after_min(leaders: &[Flamekeeper], min_leaders: usize) {
    for (i, leader) in leaders.iter().enumerate() {
        if !leader.is_leader_eligible {
            assert!(
                i < min_leaders,
                "Leader {} (Index {}) ist nicht eligible, obwohl min_leaders ({}) bereits erreicht waren.",
                leader.id,
                i,
                min_leaders
            );
        }
    }
}

pub fn print_leader_overview(leaders: &[Flamekeeper]) {
    println!("Anzahl gewählter Leader: {}", leaders.len());
    for (i, leader) in leaders.iter().enumerate() {
        println!(
            "→ [{}] Leader {} | Stake: {} | Eligible: {}",
            i, leader.id, leader.stake, leader.is_leader_eligible
        );
    }
}

pub fn generate_random_validators(count: usize) -> Vec<Flamekeeper> {
    let mut rng = thread_rng();
    (0..count)
        .map(|i| {
            let mut v = Flamekeeper::new(
                i as u64,
                rng.gen_range(1000..20_000), // Zufälliger Stake
                rng.gen_range(-180.0..180.0), // Zufällige Longitude
                rng.gen_range(-90.0..90.0),  // Zufällige Latitude
                6.0,                         // CPU units (z. B. 6 GHz)
                4.0,                         // RAM (4 GB)
                50.0,                        // Bandwidth (50 Mbps)
            );
            v.is_leader_eligible = true; // Sicherstellen, dass sie leaderfähig sind
            v
        })
        .collect()
}

pub fn run_block_simulation<F>(
    block_height: u64,
    tx_volume: f64,
    mut leader_selector: F,
    validators: &[Flamekeeper],
    tps: f64,
    active_wallets: usize,
    mev_potential: f64,
) -> Vec<Flamekeeper>
where
    F: FnMut(&[Flamekeeper], u64, f64, f64, usize, f64) -> Vec<Flamekeeper>,
{
    println!("--- Simuliere Block {} ---", block_height);
    let leaders = leader_selector(validators, block_height, tx_volume, tps, active_wallets, mev_potential);
    print_leader_overview(&leaders);
    leaders
}

pub fn generate_validators(count: usize, stake: u64) -> Vec<Flamekeeper> {
    (0..count)
        .map(|i| {
            let mut v = Flamekeeper::new(
                i as u64,
                stake,
                i as f64,
                i as f64,
                6.0,  // CPU units
                4.0,  // RAM
                50.0, // Bandwidth
            );
            v.is_leader_eligible = true;
            v
        })
        .collect()
}

pub fn generate_batch(size: usize) -> Vec<Flamecall> {
    (0..size)
        .map(|i| Flamecall::new(i as u64, "Alice", "Bob", 100, 100_000))
        .collect()
}