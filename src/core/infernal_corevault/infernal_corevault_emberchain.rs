// src/core/infernal_corevault/infernal_corevault_emberchain.rs
use crate::core::Emberblock;

pub struct Emberchain {
    blocks: Vec<Emberblock>,
}

impl Emberchain {
    pub fn new() -> Self {
        Emberchain { blocks: Vec::new() }
    }

    pub fn add_block(&mut self, block: Emberblock) {
        self.blocks.push(block);
    }

    pub fn get_blocks(&self) -> &[Emberblock] {
        &self.blocks
    }
}