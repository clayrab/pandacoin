use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::crypto::sign_message;
use crate::keypair_store::KeypairStore;
use crate::panda_protos::MiniBlockProto;
use crate::timestamp_generator::AbstractTimestampGenerator;
use crate::types::Sha256Hash;
use crate::{mempool::AbstractMempool, utxoset::AbstractUtxoSet};

#[derive(Debug)]
struct MiniblockManagerContext {
    _utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
    #[allow(dead_code)]
    mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
    #[allow(dead_code)]
    timestamp_generator: Arc<Box<dyn AbstractTimestampGenerator>>,
    keypair_store: Arc<KeypairStore>,
}

#[derive(Debug)]
pub struct MiniblockManager {
    context: MiniblockManagerContext,
}

impl MiniblockManager {
    pub fn new(
        _utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
        timestamp_generator: Arc<Box<dyn AbstractTimestampGenerator>>,
        keypair_store: Arc<KeypairStore>,
    ) -> Self {
        MiniblockManager {
            context: MiniblockManagerContext {
                _utxoset_ref,
                mempool_ref,
                timestamp_generator,
                keypair_store,
            },
        }
    }
    pub async fn new_miniblock_from_difference(
        &self, their_set: &HashSet<Sha256Hash>,
    ) -> MiniBlockProto {
        //     let mini_block_proto = MiniBlockProto {
        //         receiver: keypair_1.get_public_key().serialize().to_vec(),
        //         creator: keypair_2.get_public_key().serialize().to_vec(),
        //         signature: signature.to_vec(),
        //         merkle_root: [4; 32].to_vec(),
        //         transactions: vec![transaction_proto],
        //     };
        let mempool = self.context.mempool_ref.read().await;
        let mut current_set = mempool.get_current_set().clone();
        //let transaction_set: HashSet<&Sha256Hash> = current_set.difference(their_set).collect();
        current_set.retain(|elem| !their_set.contains(elem));
        //let transactions_without_conflict = TransactionSet::from_difference(current_set, transaction_set);
        // get transactions from mempool
        MiniBlockProto {
            receiver: [0; 32].to_vec(),
            signature: sign_message(
                &[0; 32],
                self.context.keypair_store.get_keypair().get_secret_key(),
            )
            .to_vec(),
            creator: self
                .context
                .keypair_store
                .get_keypair()
                .get_public_key()
                .serialize()
                .to_vec(),
            merkle_root: vec![],
            transactions: vec![],
        }
    }
}
