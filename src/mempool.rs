use crate::block::RawBlock;
use crate::Error;
use crate::transaction::Transaction;
use crate::utxoset::AbstractUtxoSet;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::time::sleep;

//#[async_trait]
pub trait AbstractMempool: Debug {}

#[derive(Debug)]
struct MempoolContext {
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
}

// a transaction is valid if all of it's outputs are unspent.
// if a transaction gets 'rolled back', presumably it was from a valid block beforehand and we can safely add
// the transactions back into the mempool.
// The complicated tx are the ones that were removed from the mempool because another transaction used one of their
// inputs, yet they could be re-added to mempool on rollback...
// For now we will keep these in memory and evict them after a certain amount of time. In the future, it would be better to
// have some sort of memory management which evicts different sorts of data based on how memory-constrained the system is.
// 

// MempoolState: SpentInBlock, OutputSpentInBlock, Valid

#[derive(Debug)]
pub struct Mempool {
    context: MempoolContext,
    // outputid -> transaction
    // transction -> state
}

impl AbstractMempool for Mempool {}

impl Mempool {
    pub async fn run(_shutdown_waiting_sender: mpsc::Sender<()>) -> Result<(), Error>{
        loop {
            println!("tick");
            sleep(Duration::from_millis(1000)).await;
        }
    }
    pub async fn new(utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>, mut shutdown_channel_receiver: broadcast::Receiver<()>, shutdown_waiting_sender: mpsc::Sender<()>) -> Self {

        tokio::spawn(async move {
            let result = tokio::select! {
                // Do not clone this sender, we are using it for reference counting during graceful shutdown
                res = Mempool::run(shutdown_waiting_sender) => {
                    res
                },
                _ = shutdown_channel_receiver.recv() => {
                    println!("Received shutdown signal in Mempool");
                    Ok(())
                },
            };
            println!("Mempool result: {:?}", result);
        });

        Mempool {
            context: MempoolContext { utxoset_ref },
        }
    }
    pub fn add_transaction(&mut self, _transaction: Transaction) {
        
    }

    pub fn roll_forward(&mut self, block: &Box<dyn RawBlock>) {
        println!("mempool roll_forward: {:?}", block.get_hash())
        // 
    }

    pub fn roll_back(&self, block: &Box<dyn RawBlock>) {
        println!("mempool roll_back: {:?}", block.get_hash())
        
    }
}
