use std::collections::{HashMap, HashSet};

use crate::block::RawBlock;
use crate::mempool::AbstractMempool;
use crate::panda_protos::OutputIdProto;
use crate::transaction::Transaction;
use crate::types::Sha256Hash;
use async_trait::async_trait;

#[derive(Debug)]
pub struct MockMempool {
    broker_set: HashSet<Sha256Hash>,
    transactions: HashMap<Sha256Hash, Transaction>,
    known_inputs: HashMap<OutputIdProto, HashSet<Sha256Hash>>,
}

impl MockMempool {
    pub fn new() -> Self {
        MockMempool {
            broker_set: HashSet::new(),
            transactions: HashMap::new(),
            known_inputs: HashMap::new(),
        }
    }
}

#[async_trait]
impl AbstractMempool for MockMempool {
    fn get_latest_block_id(&self) -> u32 {
        1
    }
    fn get_known_inputs(&self) -> &HashMap<OutputIdProto, HashSet<Sha256Hash>> {
        &self.known_inputs
    }
    fn get_transaction(&self, hash: &Sha256Hash) -> Option<&Transaction> {
        self.transactions.get(hash)
    }
    fn get_broker_set(&self) -> &HashSet<Sha256Hash> {
        &self.broker_set
    }
    async fn add_transaction(&mut self, _transaction: Transaction) -> bool {
        true
    }
    async fn roll_forward(&mut self, _block: &Box<dyn RawBlock>) {}
    fn roll_back(&mut self, _block: &Box<dyn RawBlock>) {}
    fn roll_forward_max_reorg(&mut self, _block: &Box<dyn RawBlock>) {}
}
