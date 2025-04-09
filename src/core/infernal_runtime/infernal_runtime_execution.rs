// src/core/infernal_runtime/infernal_runtime_execution.rs

use super::InfernalRuntime;
use crate::core::{EmberConfig, Flamecall};

pub fn execute_transaction(runtime: &mut InfernalRuntime, tx: &Flamecall, config: &EmberConfig) -> Result<(), String> {
    if tx.gas < (config.gas_base_fee * 1_000_000.0) as u64 {
        return Err("Insufficient gas for transaction".to_string());
    }

    let sender_balance = runtime.balances.entry(tx.sender.clone()).or_insert(0);
    if *sender_balance < tx.amount {
        return Err("Insufficient balance".to_string());
    }
    *sender_balance -= tx.amount;
    *runtime.balances.entry(tx.receiver.clone()).or_insert(0) += tx.amount;

    runtime.increment_block();

    log::debug!(
        "Executed transaction ID {}: {} -> {} ({} INF) at block height {}",
        tx.id, tx.sender, tx.receiver, tx.amount, runtime.current_block_height
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_transaction_success() {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut runtime = InfernalRuntime::new();
        runtime.balances.insert("Alice".to_string(), 1000); // Initialer Saldo
        let config = EmberConfig::default();
        let tx = Flamecall::new(1, "Alice", "Bob", 100, 100_000);
        assert!(execute_transaction(&mut runtime, &tx, &config).is_ok());
        assert_eq!(runtime.current_block_height, 1);
        assert_eq!(*runtime.balances.get("Alice").unwrap(), 900);
        assert_eq!(*runtime.balances.get("Bob").unwrap(), 100);
    }

    #[test]
    fn test_execute_transaction_insufficient_gas() {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut runtime = InfernalRuntime::new();
        let config = EmberConfig::default();
        let tx = Flamecall::new(1, "Alice", "Bob", 100, 1);
        assert!(execute_transaction(&mut runtime, &tx, &config).is_err());
        assert_eq!(runtime.current_block_height, 0);
    }

    #[test]
    fn test_execute_transaction_insufficient_balance() {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut runtime = InfernalRuntime::new();
        runtime.balances.insert("Alice".to_string(), 50); // Zu wenig Saldo
        let config = EmberConfig::default();
        let tx = Flamecall::new(1, "Alice", "Bob", 100, 100_000);
        assert!(execute_transaction(&mut runtime, &tx, &config).is_err());
        assert_eq!(runtime.current_block_height, 0);
        assert_eq!(*runtime.balances.get("Alice").unwrap(), 50);
    }
}