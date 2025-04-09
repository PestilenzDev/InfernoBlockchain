// src/core/infernal_neuroswarm_consensus/infernal_neuroswarm_consensus_bloom.rs

use crate::core::infernal_neuroswarm_consensus::infernal_neuroswarm_consensus_types::INSCFlamecall;
use crate::core::error::InfernoError;
use bloomfilter::Bloom;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct BloomScheduler {
    bloom: Bloom<String>,
    backoff_queue: HashMap<String, Instant>,
    false_positive_rate: f64,
    backoff_duration: Duration,
    tx_sender: mpsc::Sender<INSCFlamecall>,
    tx_receiver: mpsc::Receiver<INSCFlamecall>,
}

impl BloomScheduler {
    #[inline]
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Result<Self, InfernoError> {
        let (tx_sender, tx_receiver) = mpsc::channel(100);
        let bs = Self {
            bloom: Bloom::new_for_fp_rate(expected_items, false_positive_rate),
            backoff_queue: HashMap::new(),
            false_positive_rate,
            backoff_duration: Duration::from_millis(10),
            tx_sender,
            tx_receiver,
        };
        info!("Initialized BloomScheduler with expected_items={}, false_positive_rate={}", expected_items, false_positive_rate);
        Ok(bs)
    }

    #[inline]
    pub async fn schedule_aggregation(&mut self, tx: &INSCFlamecall, quorum_confirmed: bool) -> Result<bool, InfernoError> {
        let tx_hash = format!("{:x}", tx.core.id);

        if let Some(backoff_time) = self.backoff_queue.get(&tx_hash) {
            if Instant::now() >= *backoff_time {
                self.backoff_queue.remove(&tx_hash);
            } else {
                debug!("Tx {} in backoff, skipping aggregation", tx_hash);
                return Ok(false);
            }
        }

        if self.bloom.check(&tx_hash) && !quorum_confirmed {
            self.retry_aggregation(&tx_hash).await?;
            return Ok(false);
        }

        self.bloom.set(&tx_hash);
        self.tx_sender.send(tx.clone()).await?;
        info!("Tx {} scheduled for aggregation", tx_hash);
        Ok(true)
    }

    #[inline]
    async fn retry_aggregation(&mut self, tx_hash: &str) -> Result<(), InfernoError> {
        let backoff_time = Instant::now() + self.backoff_duration;
        self.backoff_queue.insert(tx_hash.to_string(), backoff_time);
        warn!("Tx {} marked for retry with backoff ({}ms)", tx_hash, self.backoff_duration.as_millis());
        Ok(())
    }

    #[inline]
    pub async fn process_queue(&mut self) -> Result<Vec<INSCFlamecall>, InfernoError> {
        let mut aggregated = Vec::new();
        while let Some(tx) = self.tx_receiver.recv().await {
            aggregated.push(tx);
        }
        Ok(aggregated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bloom_scheduler() {
        let mut bs = BloomScheduler::new(1000, 0.01).unwrap();
        let core_tx = CoreFlamecall::new(1, "Alice", "Bob", 100, 100_000, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(), None, None).unwrap();
        let tx = INSCFlamecall::new(core_tx, 1.0).unwrap();
        assert!(bs.schedule_aggregation(&tx, false).await.unwrap());
        assert!(!bs.schedule_aggregation(&tx, false).await.unwrap()); // Sollte in Backoff gehen
        let queued = bs.process_queue().await.unwrap();
        assert_eq!(queued.len(), 1);
    }
}