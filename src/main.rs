// src/main.rs

use inferno_blockchain::start_inferno;

fn main() {
    // Initialisiere den Logger
    env_logger::init();

    if let Err(e) = start_inferno() {
        eprintln!("Failed to start Inferno: {}", e);
        std::process::exit(1);
    } else {
        println!("Inferno Blockchain started successfully!");
    }
}