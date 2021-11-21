use std::sync::Arc;
use tokio::sync::RwLock;
use std::fmt::Debug;
use crate::utxoset::AbstractUtxoSet;


//#[async_trait]
pub trait AbstractMempool: Debug {
}

#[derive(Debug)]
struct MempoolContext {
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,   
}

#[derive(Debug)]
pub struct Mempool {
    context: MempoolContext,
}

impl AbstractMempool for Mempool {
    
}

impl Mempool {
    /// Create new `Blockchain`
    pub fn new(
        utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        
    ) -> Self {
        Mempool {
            
            context: MempoolContext {
                utxoset_ref
            },
        }
    }
}
