use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{utxoset::AbstractUtxoSet, mempool::AbstractMempool};

#[derive(Debug)]
struct MiniblockManagerContext {
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
    mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>
}

#[derive(Debug)]
pub struct MiniblockManager {
    context: MiniblockManagerContext,
}

impl MiniblockManager {
    /// Create new `Blockchain`
    pub fn new(
        utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>
        
    ) -> Self {
        MiniblockManager {
            
            context: MiniblockManagerContext {
                utxoset_ref,
                mempool_ref,
            },
        }
    }
}