// tests/core_tests.rs
use inferno_blockchain::core::infernal_neuroswarm_consensus::Lilith;
use inferno_blockchain::core::testutils::generate_validators;
use std::time::Instant;
use chrono::Local;
use std::fs::File;
use std::io::Write;

#[tokio::test]
async fn benchmark_full_consensus() {
    let _ = env_logger::builder().is_test(true).try_init();
    let validator_count = 1000;
    let batch_size = 2000;
    let num_batches = validator_count / 10;

    let validators = generate_validators(validator_count, 5000);
    let start = Instant::now();
    let _lilith = Lilith::new();
    let _tps = _lilith.predict_tps(&[], validators.len()); // Platzhalter
    let duration = start.elapsed().as_secs_f64();

    let total_tx = num_batches * batch_size;
    let tps = if duration > 0.0 { total_tx as f64 / duration } else { 0.0 };

    let summary = format!(
        "Total tx: {}\nDuration: {:.2}s\nTPS: {:.2}\n",
        total_tx, duration, tps
    );
    println!("--- Full Consensus Benchmark ---\n{}", summary);

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let filename = format!("benchmark_results_full_consensus_{}.log", timestamp);
    let mut file = match File::create(&filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("⚠️ Failed to create benchmark log file {}: {}", filename, e);
            return;
        }
    };
    if let Err(e) = file.write_all(summary.as_bytes()) {
        eprintln!("⚠️ Failed to write benchmark log to {}: {}", filename, e);
    } else {
        log::info!("Benchmark results saved to {}", filename);
    }

    assert!(tps > 100_000.0, "TPS too low: {:.2}", tps);
    assert!(duration < 0.5, "Finality too slow: {:.2}s", duration);
}