use std::{sync::Arc, collections::HashSet};
use tokio::sync::RwLock;

use crate::{mempool::AbstractMempool, utxoset::AbstractUtxoSet, types::Sha256Hash};

#[derive(Debug)]
struct MiniblockManagerContext {
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
    mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
}

#[derive(Debug)]
pub struct MiniblockManager {
    context: MiniblockManagerContext,
}

pub struct Miniblock {
    inputs: HashSet<Sha256Hash>,
}

impl MiniblockManager {
    /// Create new `Blockchain`
    pub fn new(
        utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
    ) -> Self {
        MiniblockManager {
            context: MiniblockManagerContext {
                utxoset_ref,
                mempool_ref,
            },
        }
    }
    pub fn roll_forward(&self) {
        
    }

    pub fn roll_back(&self) {
        
    }

    pub fn a_not_b(_miniblock_a: Miniblock, _miniblock_b: Miniblock) -> Miniblock {
        Miniblock {
            inputs: HashSet::new()
        }
    }
}

impl Miniblock {
    pub fn serialize(&self) {
        
    }
    
    pub fn deserialize() -> Self {
        Miniblock {
            inputs: HashSet::new()
        }
    }
}