use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::block::RawBlock;
use crate::constants::Constants;
use crate::crypto::verify_bytes_message;
use crate::keypair_store::KeypairStore;
use crate::merkle_tree_manager::MerkleTree;
use crate::miniblock::MiniBlock;
use crate::panda_protos::{OutputIdProto, RawBlockProto};
use crate::timestamp_generator::AbstractTimestampGenerator;
use crate::transaction::Transaction;
use crate::types::Sha256Hash;
use crate::{mempool::AbstractMempool, utxoset::AbstractUtxoSet};

/// Although it is technically possible to allow MiniBlocks which conflict with
///
#[derive(Debug, PartialEq, Clone)]
pub enum AddMiniBlockEvent {
    Accepted,
    Unspendable,
    MempoolConflict,
    MiniblockMempoolConflict,
    InvalidSignature,
}

#[derive(Debug)]
struct MiniblockMempoolContext {
    constants: Arc<Constants>,
    #[allow(dead_code)]
    timestamp_generator: Arc<Box<dyn AbstractTimestampGenerator + Send + Sync>>,
    keypair_store: Arc<KeypairStore>,
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
    #[allow(dead_code)]
    mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
}

/// This structure will keep track of the known miniblocks which have been sent by other
/// nodes and will invalidate them when any of their transactions are used.
#[derive(Debug)]
pub struct MiniblockMempool {
    known_inputs: HashSet<OutputIdProto>,
    mini_blocks: Vec<MiniBlock>,
    /// stores the block id which invalidated this mini_block by mini_block hash
    unspendable_mini_block_hashes: HashMap<Sha256Hash, u32>,
    total_fees: u64,
    block_id: u32,
    context: MiniblockMempoolContext,
}

impl MiniblockMempool {
    pub fn new(
        constants: Arc<Constants>,
        timestamp_generator: Arc<Box<dyn AbstractTimestampGenerator + Send + Sync>>,
        keypair_store: Arc<KeypairStore>,
        utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
    ) -> Self {
        MiniblockMempool {
            known_inputs: HashSet::new(),
            mini_blocks: vec![],
            unspendable_mini_block_hashes: HashMap::new(),
            total_fees: 0,
            block_id: 0,
            context: MiniblockMempoolContext {
                constants,
                timestamp_generator,
                keypair_store,
                utxoset_ref,
                mempool_ref,
            },
        }
    }
    pub fn get_total_fees(&self) -> u64 {
        self.total_fees
    }
    /// get known inputs. Known inputs are in a miniblock which has been sent to us.
    pub fn get_known_inputs(&self) -> &HashSet<OutputIdProto> {
        &self.known_inputs
    }
    /// get mini_blocks
    pub fn get_mini_blocks(&self) -> &Vec<MiniBlock> {
        &self.mini_blocks
    }
    /// get unspendable_mini_block_hashes
    pub fn get_unspendable_mini_block_hashes(&self) -> &HashMap<Sha256Hash, u32> {
        &self.unspendable_mini_block_hashes
    }
    /// Some of the mini_blocks in unspendable_mini_block_hashes may have become spendable.
    /// If so, remove them from unspendable_mini_block_hashes and add all their inputs
    /// back into known_inputs and fees into total_fees.
    pub async fn roll_back(&mut self, block: &Box<dyn RawBlock>) {
        assert_eq!(self.block_id, block.get_id());
        assert_ne!(0, block.get_id());
        self.block_id -= 1;

        let utxoset = self.context.utxoset_ref.read().await;
        // If a miniblock is still spendable and it doesn't conflict with the mempool, it's hash can be
        // removed from unspendable_mini_block_hashes.
        let mut mini_blocks_iter = self.mini_blocks.iter();
        while let Some(mini_block) = mini_blocks_iter.next() {
            if self
                .unspendable_mini_block_hashes
                .get(mini_block.get_hash())
                .is_some()
            {
                // unspendable transactions should not be able to get into mempool, so a block being rolled back
                // should not be able to conflict with mempool.
                // Also, all the inputs must be spendable w.r.t. the longest chain if a block is being rolled back...
                // TODO Could both of these conditionals be asserted instead?
                if self
                    .validate_miniblock_transactions_spendability(&mini_block, self.block_id)
                    .await
                {
                    if self.validate_miniblock_against_mempool(&mini_block).await {
                        // insert inputs
                        mini_block.get_transactions().iter().for_each(|tx| {
                            for input in tx.get_inputs() {
                                self.known_inputs.insert(input.clone());
                            }
                        });
                        // add the total fees
                        let block_fee = mini_block
                            .get_transactions()
                            .iter()
                            .map(|tx| utxoset.get_fee_of_transaction(tx.get_transaction_proto()))
                            .reduce(|tx_fees_a, tx_fees_b| tx_fees_a + tx_fees_b)
                            .unwrap();
                        self.total_fees += block_fee;
                        self.unspendable_mini_block_hashes
                            .remove(mini_block.get_hash());
                    }
                }
            }
        }
    }
    /// Needs to make sure all the existing mini_blocks are still spendable.
    /// If unspendable, remove all inputs from known inputs.
    pub async fn roll_forward(&mut self, block: &Box<dyn RawBlock>) {
        // Miniblocks will become "invalid" at this point, but we dont' want to remove them yet. instead we
        // just want to remove them from the total_fees and mark them as unspendable by moving them from
        // mini_blocks into unspendable_mini_blocks.
        if block.get_id() != 0 {
            self.block_id += 1;
            assert_eq!(self.block_id, block.get_id());
        }
        let utxoset = self.context.utxoset_ref.read().await;
        let mut mempool_mini_blocks_iter = self.mini_blocks.iter();
        while let Some(mini_block) = mempool_mini_blocks_iter.next() {
            // don't do anything about this mini_block if it's already been added to unspendable_mini_block_hashes
            if !self
                .unspendable_mini_block_hashes
                .get(mini_block.get_hash())
                .is_some()
            {
                let is_mini_block_spendable = self
                    .validate_miniblock_transactions_spendability(mini_block, self.block_id)
                    .await;
                if !is_mini_block_spendable {
                    // mark as unspendable and remove it's contirbution from everything.
                    // remove all; inputs from from known_inputs
                    mini_block.get_transactions().iter().for_each(|tx| {
                        for input in tx.get_inputs() {
                            self.known_inputs.remove(input);
                        }
                    });
                    // remove the fee from total_fees
                    let block_fee = mini_block
                        .get_transactions()
                        .iter()
                        .map(|tx| utxoset.get_fee_of_transaction(tx.get_transaction_proto()))
                        .reduce(|tx_fees_a, tx_fees_b| tx_fees_a + tx_fees_b)
                        .unwrap();
                    self.total_fees -= block_fee;
                    // insert the mini_block into unspendable_mini_blocks
                    self.unspendable_mini_block_hashes
                        .insert(*mini_block.get_hash(), block.get_id());
                }
            }
        }
    }
    /// if a mini_block which is already in unspendable mini_blocks becomes fully unspendable because it is not
    /// even spendable at the reorg depth, we remove it from unspendable_mini_blocks.
    pub async fn roll_forward_max_reorg(&mut self, _current_block: &Box<dyn RawBlock>) {
        // At this point, we want to expire any unspendable mini_blocks if they are not even spendable at MAX_REORG prior to latest block.
        if self.block_id >= self.context.constants.get_max_reorg() {
            let mut i = 0;
            while i < self.mini_blocks.len() {
                if let Some(mini_block) = self.mini_blocks.get(i) {
                    if let Some(block_id) = self
                        .unspendable_mini_block_hashes
                        .get(mini_block.get_hash())
                    {
                        if *block_id <= self.block_id - self.context.constants.get_max_reorg() {
                            self.unspendable_mini_block_hashes
                                .remove(self.mini_blocks[i].get_hash());
                            self.mini_blocks.swap_remove(i);
                            break;
                        }
                    }
                }
                i += 1;
            }
        }
    }

    /// make a block from the spendable mini-blocks
    pub async fn create_block(&mut self, prev_block: &Box<dyn RawBlock>) -> RawBlockProto {
        let mut mini_blocks = vec![];
        let mini_blocks_iter = self.mini_blocks.iter();
        for mini_block in mini_blocks_iter {
            if !self
                .unspendable_mini_block_hashes
                .get(mini_block.get_hash())
                .is_some()
            {
                mini_blocks.push(mini_block.clone().into_proto());
            }
        }

        let our_pubkey = self.context.keypair_store.get_keypair().get_public_key();
        let block = RawBlockProto {
            id: prev_block.get_id() + 1,
            timestamp: self.context.timestamp_generator.get_timestamp(),
            previous_block_hash: prev_block.get_hash().to_vec(),
            creator: our_pubkey.serialize().to_vec(),
            signature: vec![],
            merkle_root: vec![],
            mini_blocks: mini_blocks,
        };
        block
    }
    /// when requesting a miniblock from another peer we need to let the peer know about inputs
    /// which are in either the tx mempool or the miniblock mempool
    pub async fn get_known_input_set(&self) -> HashSet<OutputIdProto> {
        // TODO
        // return the union of known inputs from self and mempool
        let foo: HashSet<OutputIdProto> = HashSet::new();
        foo
    }

    /// Add a mini block to the mempool and track all the output hashes associated with it.
    pub async fn add_miniblock(&mut self, mini_block: MiniBlock) -> AddMiniBlockEvent {
        // TODO validate miniblock's signature is correct.
        if !self
            .validate_miniblock_against_miniblock_mempool(&mini_block)
            .await
        {
            return AddMiniBlockEvent::MiniblockMempoolConflict;
        }
        if !self
            .validate_miniblock_transactions_spendability(&mini_block, self.block_id)
            .await
        {
            return AddMiniBlockEvent::Unspendable;
        }
        if !self.validate_miniblock_against_mempool(&mini_block).await {
            return AddMiniBlockEvent::MempoolConflict;
        }
        if !self.validate_miniblock_signature(&mini_block).await {
            return AddMiniBlockEvent::InvalidSignature;
        }

        mini_block.get_transactions().iter().for_each(|tx| {
            self.add_transaction(tx.clone());
        });
        let utxoset = self.context.utxoset_ref.read().await;
        let block_fee = mini_block
            .get_transactions()
            .iter()
            .map(|tx| utxoset.get_fee_of_transaction(tx.get_transaction_proto()))
            .reduce(|tx_fees_a, tx_fees_b| tx_fees_a + tx_fees_b)
            .unwrap();
        self.total_fees += block_fee;
        self.mini_blocks.push(mini_block);
        AddMiniBlockEvent::Accepted
    }
    /// A miniblock can be spendable but also conflict with the mempool...
    /// For the time being, we will simply treat the mempool as the highest priority.
    /// If we allow mini-blocks to purge transactions from the mempool, this may create
    /// an attack vector whereby a node which we've requested a mini-block from can
    /// purposefully remove transactions from our mempool. However, it would be nice if
    /// we could at least allow transactions to be purged if they were received after
    /// the mini-block was requested(i.e. the collision was a timing accident...)
    ///
    /// Perhaps a simple mechanism for this would be to simply hold back transctions for a
    /// few seconds in a queue before adding them to mempool...?
    ///
    async fn validate_miniblock_against_mempool(&self, mini_block: &MiniBlock) -> bool {
        let mempool = self.context.mempool_ref.read().await;
        futures::stream::iter(mini_block.get_transactions())
            .all(|tx| {
                futures::stream::iter(tx.get_inputs())
                    .all(|input| async { !mempool.get_known_inputs().contains_key(input) })
            })
            .await
    }
    ///
    /// validate_miniblock_against_miniblock_mempool
    async fn validate_miniblock_against_miniblock_mempool(&self, mini_block: &MiniBlock) -> bool {
        futures::stream::iter(mini_block.get_transactions())
            .all(|tx| {
                futures::stream::iter(tx.get_inputs())
                    .all(|input| async { !self.known_inputs.contains(input) })
            })
            .await
    }
    /// validate_miniblock_signature
    async fn validate_miniblock_signature(&self, mini_block: &MiniBlock) -> bool {
        let merkle_tree = MerkleTree::new_from_hashes(
            mini_block.get_transactions().iter().map(|tx| tx.get_hash()),
        );
        let message = [
            merkle_tree.get_root().unwrap().to_vec(),
            mini_block.get_receiver().serialize().to_vec(),
        ]
        .concat();
        if let Ok(signature) = mini_block.get_signature() {
            verify_bytes_message(
                &message,
                &signature.serialize_compact().to_vec(),
                &self
                    .context
                    .keypair_store
                    .get_keypair()
                    .get_public_key()
                    .serialize()
                    .to_vec(),
            )
        } else {
            false
        }
    }

    /// validate_miniblock_transactions. make sure all the transactions are still spendable at a given block id.
    async fn validate_miniblock_transactions_spendability(
        &self,
        mini_block: &MiniBlock,
        block_id: u32,
    ) -> bool {
        let utxoset = self.context.utxoset_ref.read().await;
        futures::stream::iter(mini_block.get_transactions())
            .all(|transaction| {
                Transaction::validate_transaction(&utxoset, block_id, transaction, None)
            })
            .await
    }

    /// add all outputs in the transaction to known outputs
    fn add_transaction(&mut self, tx: Transaction) {
        for input in tx.get_inputs() {
            self.known_inputs.insert(input.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use clap::Clap;
    use tokio::sync::RwLock;

    use crate::{
        block::{PandaBlock, RawBlock},
        command_line_opts::CommandLineOpts,
        constants::Constants,
        keypair::Keypair,
        keypair_store::KeypairStore,
        mempool::AbstractMempool,
        miniblock::MiniBlock,
        miniblock_mempool::AddMiniBlockEvent,
        panda_protos::{
            transaction_proto::TxType, MiniBlockProto, OutputIdProto, OutputProto, TransactionProto,
        },
        test_utilities::{
            globals_init::make_timestamp_generator_for_test,
            mock_block::MockRawBlockForMiniBlockManager, mock_mempool::MockMempool,
        },
        timestamp_generator::AbstractTimestampGenerator,
        transaction::Transaction,
        utxoset::{AbstractUtxoSet, UtxoSet},
    };

    use super::MiniblockMempool;

    #[tokio::test]
    async fn miniblock_mempool_test() {
        // create a bunch of mock objects
        let constants = Arc::new(Constants::new_for_test(
            None,
            Some(20000),
            None,
            None,
            None,
            Some(5),
            None,
        ));

        let timestamp_generator: Arc<Box<dyn AbstractTimestampGenerator + Send + Sync>> =
            make_timestamp_generator_for_test();
        let mock_mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(MockMempool::new())));
        let command_line_opts = Arc::new(CommandLineOpts::parse_from(&[
            "pandacoin",
            "--password",
            "asdf",
        ]));
        let keypair_store = Arc::new(KeypairStore::new_mock(command_line_opts));
        let utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(UtxoSet::new(constants.clone()))));

        let keypair_1 = Keypair::new();
        let keypair_2 = Keypair::new();

        // object under test: miniblock_mempool
        let mut miniblock_mempool = MiniblockMempool::new(
            constants.clone(),
            timestamp_generator.clone(),
            keypair_store.clone(),
            utxoset_ref.clone(),
            mock_mempool_ref.clone(),
        );

        // make a first block to seed some coins so we can make valid inputs
        let transaction_proto = TransactionProto {
            timestamp: timestamp_generator.get_timestamp(),
            inputs: vec![],
            outputs: vec![OutputProto::new(*keypair_1.get_public_key(), 2000)],
            txtype: TxType::Seed as i32,
            message: vec![],
            signature: vec![0; 64],
            broker: vec![0; 32],
        };

        let mini_block_proto_0 = MiniBlockProto::new(
            vec![Transaction::from_proto(transaction_proto)],
            keypair_2.get_public_key(),
            keypair_store.get_keypair(),
        );
        let mini_block_0 = MiniBlock::from_serialiazed_proto(mini_block_proto_0.serialize());
        let mock_block_0: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            0,
            vec![mini_block_0.clone()],
        ));
        let tx_hash_0 = mini_block_0.get_transactions()[0].get_hash().clone();

        // miniblock with a bad signature should fail
        let mut mini_block_proto_bad_sig = mini_block_proto_0.clone();
        mini_block_proto_bad_sig.signature = vec![];
        let result = miniblock_mempool
            .add_miniblock(MiniBlock::from_serialiazed_proto(
                mini_block_proto_bad_sig.serialize(),
            ))
            .await;
        assert_eq!(result, AddMiniBlockEvent::InvalidSignature);

        // A good miniblock should be accepted
        let result = miniblock_mempool.add_miniblock(mini_block_0).await;
        assert_eq!(result, AddMiniBlockEvent::Accepted);
        assert_eq!(0, miniblock_mempool.get_known_inputs().len());
        assert_eq!(miniblock_mempool.get_total_fees(), 0);

        // make a 2nd block with a bunch of outputs we can use for testing
        let transaction_1 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![OutputIdProto::new(tx_hash_0.clone(), 0)],
            vec![
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 995),
            ],
            TxType::Normal,
            vec![],
            keypair_1.get_secret_key(),
        );

        let mini_block_proto_1 = MiniBlockProto::new(
            vec![transaction_1],
            keypair_1.get_public_key(),
            keypair_store.get_keypair(),
        );
        let serialized_mini_block_proto_1 = mini_block_proto_1.serialize();
        let mini_block_1 = MiniBlock::from_serialiazed_proto(serialized_mini_block_proto_1.clone());
        let mock_block_1: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            1,
            vec![mini_block_1.clone()],
        ));
        let tx_1_hash = mini_block_1.get_transactions()[0].get_hash().clone();

        // If we try to add the block now, the inputs appear invalid
        let result = miniblock_mempool.add_miniblock(mini_block_1.clone()).await;
        assert_eq!(result, AddMiniBlockEvent::Unspendable);
        // Add the first block to utxoset and miniblock 2 will become valid, should be accepted now.
        {
            let mut utxoset = utxoset_ref.write().await;
            utxoset.roll_forward(&mock_block_0);
        }
        miniblock_mempool.roll_forward(&mock_block_0).await;

        let result = miniblock_mempool.add_miniblock(mini_block_1.clone()).await;
        assert_eq!(result, AddMiniBlockEvent::Accepted);
        assert_eq!(1, miniblock_mempool.get_known_inputs().len());
        assert_eq!(5, miniblock_mempool.get_total_fees());

        // create a miniblock that conflicts with block 2
        let transaction_proto_1_conflicted = TransactionProto {
            timestamp: timestamp_generator.get_timestamp(),
            inputs: vec![OutputIdProto::new(tx_hash_0.clone(), 0)],
            outputs: vec![OutputProto::new(*keypair_1.get_public_key(), 2000)],
            txtype: TxType::Normal as i32,
            message: vec![],
            signature: vec![0; 64],
            broker: vec![0; 32],
        };
        let mini_block_proto_1_conflicted = MiniBlockProto::new(
            vec![Transaction::from_proto(transaction_proto_1_conflicted)],
            keypair_1.get_public_key(),
            keypair_store.get_keypair(),
        );
        let serialized_mini_block_proto_1_conflicted = mini_block_proto_1_conflicted.serialize();
        let mini_block_1_conflicted =
            MiniBlock::from_serialiazed_proto(serialized_mini_block_proto_1_conflicted.clone());

        // this mini block conflicts with the mempool which already contains miniblock 2
        let result = miniblock_mempool
            .add_miniblock(mini_block_1_conflicted.clone())
            .await;
        assert_eq!(result, AddMiniBlockEvent::MiniblockMempoolConflict);
        assert_eq!(1, miniblock_mempool.get_known_inputs().len());
        assert_eq!(5, miniblock_mempool.get_total_fees());
        {
            let mut utxoset = utxoset_ref.write().await;
            utxoset.roll_forward(&mock_block_1);
        }
        miniblock_mempool.roll_forward(&mock_block_1).await;

        assert_eq!(0, miniblock_mempool.get_known_inputs().len());
        assert_eq!(0, miniblock_mempool.get_total_fees());

        // If we roll this forward on utxoset it will also conflict with utxoset

        let result = miniblock_mempool
            .add_miniblock(mini_block_1_conflicted.clone())
            .await;
        assert_eq!(result, AddMiniBlockEvent::Unspendable);
        assert_eq!(0, miniblock_mempool.get_known_inputs().len());
        assert_eq!(0, miniblock_mempool.get_total_fees());

        // now that mock_block_1 has been rolled forward on utxoset, we can use it for inputs
        // create a miniblock which spents inputs 0, 1, 2, and 3

        let transaction_2_0_a = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![
                OutputIdProto::new(tx_1_hash.clone(), 0),
                OutputIdProto::new(tx_1_hash.clone(), 1),
            ],
            vec![OutputProto::new(*keypair_1.get_public_key(), 190)],
            TxType::Normal,
            vec![],
            keypair_1.get_secret_key(),
        );
        let transaction_2_0_a_hash = transaction_2_0_a.get_hash();

        let transaction_2_0_b = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![
                OutputIdProto::new(tx_1_hash.clone(), 2),
                OutputIdProto::new(tx_1_hash.clone(), 3),
            ],
            vec![OutputProto::new(*keypair_1.get_public_key(), 190)],
            TxType::Normal,
            vec![],
            keypair_1.get_secret_key(),
        );
        let transaction_2_0_b_hash = transaction_2_0_b.get_hash();

        let mini_block_proto_2_0 = MiniBlockProto::new(
            vec![transaction_2_0_a.clone(), transaction_2_0_b.clone()],
            keypair_2.get_public_key(),
            keypair_store.get_keypair(),
        );
        let serialized_mini_block_proto_2_0 = mini_block_proto_2_0.serialize();
        let mini_block_2_0 =
            MiniBlock::from_serialiazed_proto(serialized_mini_block_proto_2_0.clone());

        // A good miniblock should be accepted
        let result = miniblock_mempool
            .add_miniblock(mini_block_2_0.clone())
            .await;
        assert_eq!(result, AddMiniBlockEvent::Accepted);
        assert_eq!(4, miniblock_mempool.get_known_inputs().len());
        assert_eq!(20, miniblock_mempool.get_total_fees());

        // create a miniblock which conflicts
        let transaction_2_conflict = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![OutputIdProto::new(tx_1_hash.clone(), 1)],
            vec![OutputProto::new(*keypair_1.get_public_key(), 98)],
            TxType::Normal,
            vec![],
            keypair_1.get_secret_key(),
        );

        let mini_block_proto_2_conflict = MiniBlockProto::new(
            vec![transaction_2_conflict],
            keypair_2.get_public_key(),
            keypair_store.get_keypair(),
        );
        let serialized_mini_block_proto_2_conflict = mini_block_proto_2_conflict.serialize();
        let mini_block_2_conflict =
            MiniBlock::from_serialiazed_proto(serialized_mini_block_proto_2_conflict.clone());
        let mock_block_2_conflict: Box<dyn RawBlock> = Box::new(
            MockRawBlockForMiniBlockManager::new(2, vec![mini_block_2_conflict.clone()]),
        );

        // it can't be added to our mempool...
        let result = miniblock_mempool.add_miniblock(mini_block_2_conflict).await;
        assert_eq!(result, AddMiniBlockEvent::MiniblockMempoolConflict);

        // however, it can be rolled forward...
        // If we roll this forward on utxoset it will also conflict with utxoset
        {
            let mut utxoset = utxoset_ref.write().await;
            utxoset.roll_forward(&mock_block_2_conflict);
        }
        assert_eq!(4, miniblock_mempool.get_known_inputs().len());
        assert_eq!(20, miniblock_mempool.get_total_fees());
        // normally blockchain.rs will also roll this forward, this should invalidate
        // the mini_blocks which are in the miniblock_mempool and total_fees and known_inputs
        // will be affected.
        miniblock_mempool.roll_forward(&mock_block_2_conflict).await;
        assert_eq!(0, miniblock_mempool.get_known_inputs().len());
        assert_eq!(0, miniblock_mempool.get_total_fees());

        // rolling them back again will bring things back to the way they were
        {
            let mut utxoset = utxoset_ref.write().await;
            utxoset.roll_back(&mock_block_2_conflict);
        }
        miniblock_mempool.roll_back(&mock_block_2_conflict).await;

        assert_eq!(4, miniblock_mempool.get_known_inputs().len());
        assert_eq!(20, miniblock_mempool.get_total_fees());

        // let's add another miniblock before we make a block
        let transaction_2_1 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![
                OutputIdProto::new(tx_1_hash.clone(), 4),
                OutputIdProto::new(tx_1_hash.clone(), 5),
            ],
            vec![OutputProto::new(*keypair_1.get_public_key(), 180)],
            TxType::Normal,
            vec![],
            keypair_1.get_secret_key(),
        );

        let transaction_2_1_hash = transaction_2_1.get_hash();
        let mini_block_proto_2_1 = MiniBlockProto::new(
            vec![transaction_2_1.clone()],
            keypair_2.get_public_key(),
            keypair_store.get_keypair(),
        );
        let serialized_mini_block_proto_2_1 = mini_block_proto_2_1.serialize();
        let mini_block_2_1 =
            MiniBlock::from_serialiazed_proto(serialized_mini_block_proto_2_1.clone());

        // A good miniblock should be accepted
        let result = miniblock_mempool.add_miniblock(mini_block_2_1).await;
        assert_eq!(result, AddMiniBlockEvent::Accepted);
        assert_eq!(6, miniblock_mempool.get_known_inputs().len());
        assert_eq!(40, miniblock_mempool.get_total_fees());

        {
            let utxoset = utxoset_ref.read().await;
            let block_proto = miniblock_mempool.create_block(&mock_block_1).await;
            let block =
                PandaBlock::from_serialized_proto(block_proto.serialize().clone(), &utxoset).await;
            let tx_iter = block.transactions_iter();
            for tx in tx_iter {
                assert!(
                    tx.get_hash() == transaction_2_0_a_hash
                        || tx.get_hash() == transaction_2_0_b_hash
                        || tx.get_hash() == transaction_2_1_hash
                );
            }
        }
    }
    #[tokio::test]
    async fn miniblock_mempool_roll_forward_max_reorg_test() {
        // create a bunch of mock objects
        let constants = Arc::new(Constants::new_for_test(
            None,
            Some(20000),
            None,
            None,
            None,
            Some(5),
            None,
        ));

        let timestamp_generator: Arc<Box<dyn AbstractTimestampGenerator + Send + Sync>> =
            make_timestamp_generator_for_test();
        let mock_mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(MockMempool::new())));
        let command_line_opts = Arc::new(CommandLineOpts::parse_from(&[
            "pandacoin",
            "--password",
            "asdf",
        ]));
        let keypair_store = Arc::new(KeypairStore::new_mock(command_line_opts));
        let utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(UtxoSet::new(constants.clone()))));

        let keypair_1 = Keypair::new();
        let keypair_2 = Keypair::new();

        // object under test: miniblock_mempool
        let mut miniblock_mempool = MiniblockMempool::new(
            constants.clone(),
            timestamp_generator.clone(),
            keypair_store.clone(),
            utxoset_ref.clone(),
            mock_mempool_ref.clone(),
        );

        // make a first block to seed some coins so we can make valid inputs
        let transaction_proto = TransactionProto {
            timestamp: timestamp_generator.get_timestamp(),
            inputs: vec![],
            outputs: vec![OutputProto::new(*keypair_1.get_public_key(), 2000)],
            txtype: TxType::Seed as i32,
            message: vec![],
            signature: vec![0; 64],
            broker: vec![0; 32],
        };

        let mini_block_proto = MiniBlockProto::new(
            vec![Transaction::from_proto(transaction_proto)],
            keypair_2.get_public_key(),
            keypair_store.get_keypair(),
        );
        let mini_block = MiniBlock::from_serialiazed_proto(mini_block_proto.serialize());
        let mock_block_0: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            0,
            vec![mini_block.clone()],
        ));
        let tx_hash = mini_block.get_transactions()[0].get_hash().clone();

        // make a 2nd block with a bunch of outputs we can use for testing
        let transaction_1 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![OutputIdProto::new(tx_hash.clone(), 0)],
            vec![
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 100),
                OutputProto::new(*keypair_1.get_public_key(), 995),
            ],
            TxType::Normal,
            vec![],
            keypair_1.get_secret_key(),
        );

        let mini_block_proto_1 = MiniBlockProto::new(
            vec![transaction_1],
            keypair_1.get_public_key(),
            keypair_store.get_keypair(),
        );
        let serialized_mini_block_proto_1 = mini_block_proto_1.serialize();
        let mini_block_1 = MiniBlock::from_serialiazed_proto(serialized_mini_block_proto_1.clone());
        let mock_block_1: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            1,
            vec![mini_block_1.clone()],
        ));
        let tx_1_hash = mini_block_1.get_transactions()[0].get_hash().clone();

        // Add the first block to utxoset and miniblock 2 will become valid, should be accepted now.

        let transaction_2 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![
                OutputIdProto::new(tx_1_hash.clone(), 0),
                OutputIdProto::new(tx_1_hash.clone(), 1),
            ],
            vec![OutputProto::new(*keypair_1.get_public_key(), 190)],
            TxType::Normal,
            vec![],
            keypair_1.get_secret_key(),
        );

        let mini_block_proto_2 = MiniBlockProto::new(
            vec![transaction_2.clone()],
            keypair_2.get_public_key(),
            keypair_store.get_keypair(),
        );
        let serialized_mini_block_proto_2 = mini_block_proto_2.serialize();
        let mini_block_2 = MiniBlock::from_serialiazed_proto(serialized_mini_block_proto_2.clone());
        let mock_block_2: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            2,
            vec![mini_block_2.clone()],
        ));

        // create a miniblock which conflicts
        let transaction_2_conflict = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![OutputIdProto::new(tx_1_hash.clone(), 1)],
            vec![OutputProto::new(*keypair_1.get_public_key(), 98)],
            TxType::Normal,
            vec![],
            keypair_1.get_secret_key(),
        );

        let mini_block_proto_2_conflict = MiniBlockProto::new(
            vec![transaction_2_conflict],
            keypair_2.get_public_key(),
            keypair_store.get_keypair(),
        );
        let serialized_mini_block_proto_2_conflict = mini_block_proto_2_conflict.serialize();
        let mini_block_2_conflict =
            MiniBlock::from_serialiazed_proto(serialized_mini_block_proto_2_conflict.clone());

        // let serialized_mini_block_proto_4 = ;
        let mock_block_3: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            3,
            vec![MiniBlock::from_serialiazed_proto(
                MiniBlockProto::new(
                    vec![],
                    keypair_2.get_public_key(),
                    keypair_store.get_keypair(),
                )
                .serialize(),
            )],
        ));
        let mock_block_4: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            4,
            vec![MiniBlock::from_serialiazed_proto(
                MiniBlockProto::new(
                    vec![],
                    keypair_2.get_public_key(),
                    keypair_store.get_keypair(),
                )
                .serialize(),
            )],
        ));
        let mock_block_5: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            5,
            vec![MiniBlock::from_serialiazed_proto(
                MiniBlockProto::new(
                    vec![],
                    keypair_2.get_public_key(),
                    keypair_store.get_keypair(),
                )
                .serialize(),
            )],
        ));
        let mock_block_6: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            6,
            vec![MiniBlock::from_serialiazed_proto(
                MiniBlockProto::new(
                    vec![],
                    keypair_2.get_public_key(),
                    keypair_store.get_keypair(),
                )
                .serialize(),
            )],
        ));
        let mock_block_7: Box<dyn RawBlock> = Box::new(MockRawBlockForMiniBlockManager::new(
            7,
            vec![MiniBlock::from_serialiazed_proto(
                MiniBlockProto::new(
                    vec![],
                    keypair_2.get_public_key(),
                    keypair_store.get_keypair(),
                )
                .serialize(),
            )],
        ));

        // it can't be added to our mempool...

        {
            let mut utxoset = utxoset_ref.write().await;
            utxoset.roll_forward(&mock_block_0);
        }
        miniblock_mempool.roll_forward(&mock_block_0).await;
        miniblock_mempool
            .roll_forward_max_reorg(&mock_block_0)
            .await;

        {
            let mut utxoset = utxoset_ref.write().await;
            utxoset.roll_forward(&mock_block_1);
        }
        miniblock_mempool.roll_forward(&mock_block_1).await;
        miniblock_mempool
            .roll_forward_max_reorg(&mock_block_1)
            .await;

        let result = miniblock_mempool.add_miniblock(mini_block_2_conflict).await;
        assert_eq!(result, AddMiniBlockEvent::Accepted);

        assert_eq!(1, miniblock_mempool.get_known_inputs().len());
        assert_eq!(2, miniblock_mempool.get_total_fees());

        {
            let mut utxoset = utxoset_ref.write().await;
            utxoset.roll_forward(&mock_block_2);
        }
        miniblock_mempool.roll_forward(&mock_block_2).await;
        miniblock_mempool
            .roll_forward_max_reorg(&mock_block_2)
            .await;

        assert_eq!(0, miniblock_mempool.get_known_inputs().len());
        assert_eq!(0, miniblock_mempool.get_total_fees());

        {
            let mut utxoset = utxoset_ref.write().await;
            utxoset.roll_forward(&mock_block_3);
        }
        miniblock_mempool.roll_forward(&mock_block_3).await;
        miniblock_mempool
            .roll_forward_max_reorg(&mock_block_3)
            .await;

        miniblock_mempool.roll_forward(&mock_block_4).await;
        miniblock_mempool
            .roll_forward_max_reorg(&mock_block_4)
            .await;
        assert_eq!(
            1,
            miniblock_mempool.get_unspendable_mini_block_hashes().len()
        );
        assert_eq!(1, miniblock_mempool.get_mini_blocks().len());
        miniblock_mempool.roll_forward(&mock_block_5).await;
        miniblock_mempool
            .roll_forward_max_reorg(&mock_block_5)
            .await;

        assert_eq!(
            1,
            miniblock_mempool.get_unspendable_mini_block_hashes().len()
        );
        assert_eq!(1, miniblock_mempool.get_mini_blocks().len());

        miniblock_mempool.roll_forward(&mock_block_6).await;
        miniblock_mempool
            .roll_forward_max_reorg(&mock_block_6)
            .await;

        assert_eq!(
            1,
            miniblock_mempool.get_unspendable_mini_block_hashes().len()
        );
        assert_eq!(1, miniblock_mempool.get_mini_blocks().len());

        miniblock_mempool.roll_forward(&mock_block_7).await;
        miniblock_mempool
            .roll_forward_max_reorg(&mock_block_7)
            .await;

        assert_eq!(
            0,
            miniblock_mempool.get_unspendable_mini_block_hashes().len()
        );
        assert_eq!(0, miniblock_mempool.get_mini_blocks().len());
    }
}
