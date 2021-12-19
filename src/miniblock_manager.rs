use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

use crate::{mempool::AbstractMempool, types::Sha256Hash, utxoset::AbstractUtxoSet};

#[derive(Debug)]
struct MiniblockManagerContext {
    _utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
    _mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
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
        _utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        _mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
    ) -> Self {
        MiniblockManager {
            context: MiniblockManagerContext {
                _utxoset_ref,
                _mempool_ref,
            },
        }
    }
    pub fn roll_forward(&self) {}

    pub fn roll_back(&self) {}

    pub fn a_not_b(_miniblock_a: Miniblock, _miniblock_b: Miniblock) -> Miniblock {
        Miniblock {
            inputs: HashSet::new(),
        }
    }
}

impl Miniblock {
    pub fn serialize(&self) {}

    pub fn deserialize() -> Self {
        Miniblock {
            inputs: HashSet::new(),
        }
    }
}
