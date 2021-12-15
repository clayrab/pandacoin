use crate::transaction::Transaction;
use crate::utxoset::AbstractUtxoSet;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

//#[async_trait]
pub trait AbstractMempool: Debug {}

#[derive(Debug)]
struct MempoolContext {
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
}

#[derive(Debug)]
pub struct Mempool {
    context: MempoolContext,
}

impl AbstractMempool for Mempool {}

impl Mempool {
    /// Create new `Blockchain`
    pub fn new(utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>) -> Self {
        Mempool {
            context: MempoolContext { utxoset_ref },
        }
    }
    pub fn add_transaction(&mut self, _transaction: Transaction) {
        
    }
}
