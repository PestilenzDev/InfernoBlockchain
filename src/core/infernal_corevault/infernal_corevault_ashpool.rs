// src/core/infernal_corevault/infernal_corevault_ashpool.rs
use crate::core::Flamecall;
use std::collections::VecDeque;

pub struct Ashpool {
    transactions: VecDeque<Flamecall>,
}

impl Ashpool {
    pub fn new() -> Self {
        Ashpool { transactions: VecDeque::new() }
    }

    pub fn add_transaction(&mut self, tx: Flamecall) {
        self.transactions.push_back(tx);
    }

    pub fn pop_transaction(&mut self) -> Option<Flamecall> {
        self.transactions.pop_front()
    }
}