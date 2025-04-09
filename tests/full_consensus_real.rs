// tests/full_consensus_real.rs
use inferno_blockchain::core::{Flamecall as CoreFlamecall, Flamekeeper};
use inferno_blockchain::core::infernal_neuroswarm_consensus::{InfernalNeuroSwarmConsensus, SuperNeuron, INSCFlamecall};
use std::time::{Instant, Duration};
use tokio::time::sleep;
use rand::Rng;
use chrono::Local;
use std::fs::File;
use std::io::{Write, BufWriter};
use libp2p::identity::Keypair;
use libp2p::gossipsub::{Gossipsub, GossipsubConfigBuilder};

#[tokio::test]
async fn test_realistic_consensus_under_load() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init();

    log::info!("Starting realistic consensus test under load");
    let validator_count = 1000;
    let batch_size = 250;
    let num_batches = 800;
    let tx_volume = batch_size * num_batches;

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let error_filename = format!("benchmark_errors_full_consensus_real_{}.log", timestamp);
    let general_error_filename = "general_errors.log";
    let _error_file = BufWriter::new(File::create(&error_filename).unwrap());
    let _general_error_file = BufWriter::new(File::create(&general_error_filename).unwrap_or_else(|e| {
        eprintln!("Failed to create general error log: {}", e);
        File::create("/tmp/general_errors.log").unwrap()
    }));

    log::info!("Generating {} validators with geo-distribution", validator_count);
    let mut rng = rand::thread_rng();
    let mut validators = Vec::with_capacity(validator_count);
    for i in 0..validator_count {
        let (lat, lon) = match i % 3 {
            0 => (50.0, 10.0),  // EU
            1 => (40.0, -100.0), // NA
            _ => (35.0, 100.0),  // ASIA
        };
        let mut v = Flamekeeper::new(i as u64, 10_000, lat, lon, 6.0, 4.0, 50.0);
        v.latency = match (lat, lon) {
            (50.0, 10.0) => rng.gen_range(20..50),
            (40.0, -100.0) => rng.gen_range(50..100),
            _ => rng.gen_range(100..200),
        };
        if rng.gen_bool(0.1) { v.stake = 0; } // 10% ausfallende Validatoren
        validators.push(v);
    }
    let active_validators: Vec<_> = validators.into_iter().filter(|v| v.stake > 0).collect();
    log::info!("Generated {} active validators (stake > 0)", active_validators.len());

    let keypair = Keypair::generate_ed25519();
    let gossipsub = Gossipsub::new(keypair.into(), GossipsubConfigBuilder::default().build().unwrap()).unwrap();
    let mut consensus = InfernalNeuroSwarmConsensus::new(
        active_validators.clone(),
        gossipsub.clone(),
    );

    let mut transactions = Vec::new();
    let shards = consensus.get_shards();
    for i in 0..tx_volume {
        let sender = if rng.gen_bool(0.1) { "spammer" } else { "user" };
        let core_tx = CoreFlamecall::new(i as u64, sender, "Bob", 100, 100_000, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), None, None).unwrap();
        let mut tx = INSCFlamecall::new(core_tx, 1.0).unwrap();
        let shard_idx = i % shards.len();
        let shard_id = shards[shard_idx].id;
        let neuron_id = shards[shard_idx].neurons[rng.gen_range(0..shards[shard_idx].neurons.len())].id;
        tx.core.shard_id = Some(shard_id);
        tx.core.neuron_id = Some(neuron_id);
        transactions.push(tx);
    }
    log::info!("Generated {} transactions across {} batches", tx_volume, num_batches);

    let start = Instant::now();

    // Simuliere Chaos während des Konsens
    let mut chaos = super::super::infernal_neuroswarm_consensus_chaos::Chaos::new(0.1, gossipsub.clone()).unwrap();
    let mut shards = consensus.storage.shards.values().cloned().collect::<Vec<_>>();
    chaos.induce_partition(&mut shards).await.unwrap();
    for shard in shards {
        consensus.storage.store_shard(shard).unwrap();
    }

    sleep(Duration::from_millis(10)).await;
    let blocks = consensus.process_batch(transactions).await.unwrap();
    let duration = start.elapsed().as_secs_f64();

    let total_tx = blocks.iter().map(|b| b.core.transactions.len()).sum::<usize>();
    let approved_batches = blocks.len();
    let tps = if duration > 0.0 { total_tx as f64 / duration } else { 0.0 };
    let target_tps = tx_volume as f64 / 0.095; // Ziel: <0,095s Finalität

    let summary = format!(
        "Shards: {}\nApproved Batches: {}\nTotal tx: {}\nTotal Duration: {:.3}s\nTPS: {:.2}\nTarget TPS: {:.2}\n",
        consensus.storage.shards.len(), approved_batches, total_tx, duration, tps, target_tps
    );
    println!("--- Realistic Consensus Under Load ---\n{}", summary);

    let result_filename = format!("benchmark_results_full_consensus_real_{}.log", timestamp);
    let mut file = File::create(&result_filename).unwrap();
    file.write_all(summary.as_bytes()).unwrap();
    log::info!("Benchmark results saved to {}", result_filename);

    assert!(approved_batches > 0, "No batches approved");
    assert!(tps > target_tps * 0.95, "TPS below target: {:.2}, expected: {:.2}", tps, target_tps);
    assert!(duration < 0.095, "Finality exceeded: {:.3}s", duration);
}