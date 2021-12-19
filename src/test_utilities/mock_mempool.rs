use std::collections::HashSet;

use crate::block::RawBlock;
use crate::mempool::AbstractMempool;
use crate::transaction::Transaction;
use crate::types::Sha256Hash;
use async_trait::async_trait;

#[derive(Debug)]
pub struct MockMempool {
    current_set: HashSet<Sha256Hash>,
}

impl MockMempool {
    pub fn new() -> Self {
        MockMempool {
            current_set: HashSet::new(),
        }
    }
}

#[async_trait]
impl AbstractMempool for MockMempool {
    fn get_latest_block_id(&self) -> u32 {
        1
    }

    fn get_current_set(&self) -> &HashSet<Sha256Hash> {
        &self.current_set
    }
    async fn add_transaction(&mut self, _transaction: Transaction) -> bool {
        true
    }
    fn roll_forward(&mut self, _block: &Box<dyn RawBlock>) {}
    fn roll_back(&mut self, _block: &Box<dyn RawBlock>) {}
    fn roll_forward_max_reorg(&mut self, _block: &Box<dyn RawBlock>) {}
}
