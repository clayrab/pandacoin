use async_trait::async_trait;
use futures::StreamExt;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::blocks_database::BlocksDatabase;
use crate::constants::Constants;
use crate::fork_manager::ForkManager;
use crate::longest_chain_queue::LongestChainQueue;
use crate::mempool::AbstractMempool;
use crate::miniblock_mempool::MiniblockMempool;
use crate::transaction::Transaction;
use crate::types::Sha256Hash;
use crate::utxoset::AbstractUtxoSet;
use crate::Error;
use crate::{block::RawBlock, block_fee_manager::BlockFeeManager};

use log::{error, info};

/// Initial Treasury
pub const TREASURY: u64 = 286_810_000_000_000_000;

/// Enumerated types of `Transaction`s to be handed by consensus
#[derive(Debug, PartialEq, Clone)]
pub enum AddBlockEvent {
    AcceptedAsNewLongestChain,
    AcceptedAsLongestChain,
    Accepted,
    AlreadyKnown,
    AncestorNotFound,
    ParentNotFound,
    InvalidBlock,
    TooOld,
}

#[async_trait]
pub trait AbstractBlockchain: Debug {
    fn latest_block(&self) -> Option<&Box<dyn RawBlock>>;
    fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Box<dyn RawBlock>>;
    fn get_block_by_id(&self, block_id: u32) -> Option<&Box<dyn RawBlock>>;
    async fn add_block(&mut self, block: Box<dyn RawBlock>) -> AddBlockEvent;
    async fn remove_block(&mut self, block_hash: &Sha256Hash) -> Result<(), Error>;
}

pub struct ForkChains {
    pub ancestor_block_hash: Sha256Hash,
    pub ancestor_block_id: u32,
    pub new_chain: Vec<Sha256Hash>,
    pub old_chain: Vec<Sha256Hash>,
}
/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the blocks that is sitting off
/// the longest-chain but capable of being switched over.
#[derive(Debug)]
struct BlockchainContext {
    constants: Arc<Constants>,
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
    mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
    miniblock_mempool_ref: Arc<RwLock<Box<MiniblockMempool>>>,
}

#[derive(Debug)]
pub struct Blockchain {
    /// A queue-like structure that holds the longest chain
    longest_chain_queue: LongestChainQueue,
    /// hashmap backed tree to track blocks and potential forks
    blocks_database: BlocksDatabase,
    ///
    fork_manager: ForkManager,
    ///
    block_fee_manager: BlockFeeManager,
    ///
    context: BlockchainContext,
}

#[async_trait]
impl AbstractBlockchain for Blockchain {
    fn latest_block(&self) -> Option<&Box<dyn RawBlock>> {
        //self.blocks_database.block_by_hash(&self.longest_chain_queue.latest_block_hash())
        match self.longest_chain_queue.latest_block_hash() {
            Some(latest_block_hash) => self.blocks_database.get_block_by_hash(latest_block_hash),
            None => None,
        }
    }
    /// get a block from the blockchain by hash
    fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Box<dyn RawBlock>> {
        self.blocks_database.get_block_by_hash(block_hash)
    }
    /// get a block from the blockchain by id
    fn get_block_by_id(&self, block_id: u32) -> Option<&Box<dyn RawBlock>> {
        let block_hash = self.longest_chain_queue.get_block_hash_by_id(block_id)?;
        self.get_block_by_hash(block_hash)
    }
    /// remove blocks that are in fork branches and have become too old(beyond MAX_REORG)
    async fn remove_block(&mut self, block_hash: &Sha256Hash) -> Result<(), Error> {
        // TODO implement this
        // remove from fork tree
        info!("remove block {:?}", block_hash);
        Ok(())
    }
    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    async fn add_block(&mut self, block: Box<dyn RawBlock>) -> AddBlockEvent {
        println!(
            "***************** add block ***************** {:?}",
            block.get_hash()
        );
        // TODO: Should we pass a serialized block [u8] to add_block instead of a Block?
        if block.get_id() + self.context.constants.get_max_reorg()
            < self.longest_chain_queue.latest_block_id()
        {
            AddBlockEvent::TooOld
        } else {
            let is_first_block = block.get_previous_block_hash() == [0u8; 32]
                && !self.contains_block_hash(&block.get_previous_block_hash());
            if self.contains_block_hash(block.get_hash()) {
                AddBlockEvent::AlreadyKnown
            } else if !is_first_block && !self.contains_block_hash(&block.get_previous_block_hash())
            {
                AddBlockEvent::ParentNotFound
            } else {
                let fork_chains: ForkChains = self.find_fork_chains(&block);
                if !self.validate_block(&block, &fork_chains).await {
                    AddBlockEvent::InvalidBlock
                } else {
                    self.fork_manager
                        .roll_forward(&block, &mut self.blocks_database, &self.longest_chain_queue)
                        .await;
                    let latest_block_hash = self.longest_chain_queue.latest_block_hash();
                    let is_new_lc_tip = latest_block_hash == Some(&block.get_previous_block_hash());
                    if is_first_block || is_new_lc_tip {
                        // First Block or we'e new tip of the longest chain
                        self.longest_chain_queue.roll_forward(block.get_hash());
                        {
                            let mut utxoset = self.context.utxoset_ref.write().await;
                            utxoset.roll_forward(&block);
                        }
                        let mut mempool = self.context.mempool_ref.write().await;
                        mempool.roll_forward(&block).await;
                        let mut miniblock_mempool =
                            self.context.miniblock_mempool_ref.write().await;
                        miniblock_mempool.roll_forward(&block).await;

                        self.roll_forward_max_reorg(&block).await;
                        self.blocks_database.insert(block);
                        // self.storage.roll_forward(&block).await;
                        AddBlockEvent::AcceptedAsLongestChain
                    } else {
                        // We are on the longest chain
                        if fork_chains.new_chain.len() > fork_chains.old_chain.len() {
                            assert_eq!(
                                fork_chains.new_chain.len(),
                                fork_chains.old_chain.len() + 1
                            );
                            self.roll_forward_max_reorg(&block).await;
                            self.blocks_database.insert(block);
                            // Unwind the old chain
                            let _result = fork_chains.old_chain.iter().map(|_hash| {
                                self.longest_chain_queue.roll_back();
                            });
                            for block_hash in fork_chains.old_chain.iter().rev() {
                                let block: &Box<dyn RawBlock> =
                                    self.blocks_database.get_block_by_hash(block_hash).unwrap();
                                self.longest_chain_queue.roll_back();
                                {
                                    let mut utxoset = self.context.utxoset_ref.write().await;
                                    utxoset.roll_back(block);
                                }
                                let mut mempool = self.context.mempool_ref.write().await;
                                mempool.roll_back(block);
                                let mut miniblock_mempool =
                                    self.context.miniblock_mempool_ref.write().await;
                                miniblock_mempool.roll_back(&block).await;

                                // self.storage.roll_back(&block);
                            }

                            // Wind up the new chain
                            for block_hash in fork_chains.new_chain.iter() {
                                let block: &Box<dyn RawBlock> =
                                    self.blocks_database.get_block_by_hash(block_hash).unwrap();
                                self.longest_chain_queue.roll_forward(block.get_hash());
                                {
                                    let mut utxoset = self.context.utxoset_ref.write().await;
                                    utxoset.roll_forward(block);
                                }
                                let mut mempool = self.context.mempool_ref.write().await;
                                mempool.roll_forward(block).await;
                                let mut miniblock_mempool =
                                    self.context.miniblock_mempool_ref.write().await;
                                miniblock_mempool.roll_forward(&block).await;
                                // self.storage.roll_forward(block).await;
                            }
                            // Wind the old chain as a fork chain
                            for block_hash in fork_chains.old_chain.iter() {
                                let block: &Box<dyn RawBlock> =
                                    self.blocks_database.get_block_by_hash(block_hash).unwrap();
                                let mut utxoset = self.context.utxoset_ref.write().await;
                                utxoset.roll_forward_on_fork(block);
                                // self.storage.roll_back(&block);
                            }

                            AddBlockEvent::AcceptedAsNewLongestChain
                        } else {
                            // we're just building on a fork...
                            let mut utxoset = self.context.utxoset_ref.write().await;
                            utxoset.roll_forward_on_fork(&block);

                            self.blocks_database.insert(block);
                            AddBlockEvent::Accepted
                        }
                    }
                }
            }
        }
    }
}

impl Blockchain {
    /// Create new `Blockchain`
    pub async fn new(
        fork_manager: ForkManager,
        longest_chain_queue: LongestChainQueue,
        blocks_database: BlocksDatabase,
        block_fee_manager: BlockFeeManager,
        constants: Arc<Constants>,
        utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>>,
        miniblock_mempool_ref: Arc<RwLock<Box<MiniblockMempool>>>,
    ) -> Self {
        Blockchain {
            longest_chain_queue,
            blocks_database,
            fork_manager,
            block_fee_manager,
            context: BlockchainContext {
                constants,
                utxoset_ref,
                mempool_ref,
                miniblock_mempool_ref,
            },
        }
    }
    ///
    async fn roll_forward_max_reorg(&self, current_block: &Box<dyn RawBlock>) {
        if current_block.get_id() > self.context.constants.get_max_reorg() {
            let mut mempool = self.context.mempool_ref.write().await;
            mempool.roll_forward_max_reorg(
                self.get_block_by_id(
                    current_block.get_id() - self.context.constants.get_max_reorg(),
                )
                .unwrap(),
            );

            let mut miniblock_mempool = self.context.miniblock_mempool_ref.write().await;
            miniblock_mempool
                .roll_forward_max_reorg(current_block)
                .await;
        }
    }
    /// If the block is in the fork
    fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.blocks_database.contains_block_hash(block_hash)
    }

    fn build_new_chain(&self, this_block: &Box<dyn RawBlock>, new_chain: &mut Vec<Sha256Hash>) {
        if !self
            .longest_chain_queue
            .contains_hash_by_block_id(this_block.get_hash(), this_block.get_id())
        {
            if let Some(prev_block) = self
                .blocks_database
                .get_block_by_hash(&mut this_block.get_previous_block_hash())
            {
                self.build_new_chain(prev_block, new_chain);
            }
            new_chain.push(*this_block.get_hash());
        }
    }
    fn find_fork_chains(&self, block: &Box<dyn RawBlock>) -> ForkChains {
        let mut old_chain = vec![];
        let mut new_chain = vec![];

        self.build_new_chain(block, &mut new_chain);

        let mut first_block_in_new_chain = block;
        if &new_chain[0] != block.get_hash() {
            first_block_in_new_chain = self
                .blocks_database
                .get_block_by_hash(&new_chain[0])
                .unwrap();
        }

        let mut i = first_block_in_new_chain.get_id();
        while i < self.longest_chain_queue.latest_block_id() {
            let block = self
                .blocks_database
                .get_block_by_hash(self.longest_chain_queue.get_block_hash_by_id(i).unwrap())
                .unwrap();
            old_chain.push(*block.get_hash());
            i += 1;
        }

        let root_block = self
            .blocks_database
            .get_block_by_hash(&first_block_in_new_chain.get_previous_block_hash())
            .unwrap();
        ForkChains {
            ancestor_block_hash: *root_block.get_hash(),
            ancestor_block_id: root_block.get_id(),
            old_chain,
            new_chain,
        }
    }

    async fn validate_block(&self, block: &Box<dyn RawBlock>, fork_chains: &ForkChains) -> bool {
        // If the block has an empty hash as previous_block_hash, it's valid no question
        let previous_block_hash = block.get_previous_block_hash();
        if previous_block_hash == [0; 32] && block.get_id() == 1 {
            // if block.burnfee() != DEFAULT_BURN_FEE {
            //     return false;
            // }
            true
        } else {
            // If the previous block hash doesn't exist in the BlocksDatabase, it's rejected
            if !self
                .blocks_database
                .contains_block_hash(&previous_block_hash)
            {
                info!("invalid block, previous block not found");
                return false;
            }
            // We need previous block to validate a lot of things
            let previous_block = self
                .blocks_database
                .get_block_by_hash(&previous_block_hash)
                .unwrap();

            // The block id must be correct(+1 from previous block by hash)
            if previous_block.get_id() + 1 != block.get_id() {
                error!("Invalid block, wrong block id");
                return false;
            }
            // the block fee must be sufficient
            let next_fee = self
                .block_fee_manager
                .get_next_fee_on_fork(
                    block,
                    &self.fork_manager,
                    &self.longest_chain_queue,
                    &self.blocks_database,
                )
                .await;
            if block.get_block_fee() < next_fee {
                error!("Invalid block, insufficient fee");
                return false;
            }
            // the timestamp must be at least 1 ms greater than the previous block
            if previous_block.get_timestamp() > block.get_timestamp() {
                error!("Invalid block, timestamp must be greater than previous block");
                return false;
            }
            // the merkle root must be correctly computed

            if block.get_merkle_root() != block.get_merkle_tree().get_root().unwrap() {
                error!("Invalid block, merkle root does not match tree");
                return false;
            }
            // all the transactions must be valid
            let utxoset = self.context.utxoset_ref.read().await;
            futures::stream::iter(block.transactions_iter())
                .all(|transaction| {
                    Transaction::validate_transaction(
                        &utxoset,
                        previous_block.get_id(),
                        transaction,
                        Some(fork_chains),
                    )
                })
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AbstractBlockchain, AddBlockEvent};
    use crate::block::PandaBlock;
    use crate::block_fee_manager::BlockFeeManager;
    use crate::blocks_database::BlocksDatabase;
    use crate::command_line_opts::CommandLineOpts;
    use crate::constants::Constants;
    use crate::fork_manager::ForkManager;
    use crate::keypair_store::KeypairStore;
    use crate::longest_chain_queue::LongestChainQueue;
    use crate::mempool::AbstractMempool;
    use crate::miniblock_mempool::MiniblockMempool;
    use crate::test_utilities::mock_mempool::MockMempool;
    use crate::transaction::Transaction;
    use crate::utxoset::AbstractUtxoSet;
    use crate::{
        block::RawBlock,
        blockchain::Blockchain,
        keypair::Keypair,
        panda_protos::{transaction_proto::TxType, OutputIdProto, OutputProto},
        test_utilities::{
            globals_init::make_timestamp_generator_for_test, mock_block::MockRawBlockForBlockchain,
        },
        utxoset::UtxoSet,
    };
    use clap::Clap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn _teardown() -> std::io::Result<()> {
        let dir_path = String::from("data/test/blocks/");
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(path)?;
            }
        }
        std::fs::File::create("./src/data/blocks/empty")?;
        Ok(())
    }

    #[tokio::test]
    async fn build_new_chain_test() {
        let timestamp_generator = make_timestamp_generator_for_test();
        let constants = Arc::new(Constants::new());
        let keypair = Keypair::new();
        let genesis_block = PandaBlock::new_genesis_block(*keypair.get_public_key(), 0, 1);
        let genesis_block_hash = *genesis_block.get_hash();
        let mut longest_chain_queue = LongestChainQueue::new(&genesis_block);
        let fork_manager = ForkManager::new(&genesis_block, constants.clone());
        let block_fee_manager = BlockFeeManager::new(constants.clone());
        let mut blocks_database = BlocksDatabase::new(genesis_block);
        let utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(UtxoSet::new(constants.clone()))));
        let mock_mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(MockMempool::new())));

        let command_line_opts = Arc::new(CommandLineOpts::parse_from(&[
            "pandacoin",
            "--password",
            "asdf",
        ]));
        let keypair_store = Arc::new(KeypairStore::new_mock(command_line_opts));

        let miniblock_mempool_mutex_ref = Arc::new(RwLock::new(Box::new(MiniblockMempool::new(
            constants.clone(),
            timestamp_generator.clone(),
            keypair_store.clone(),
            utxoset_ref.clone(),
            mock_mempool_ref.clone(),
        ))));

        let mock_block_1: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            1,
            0,
            [1; 32],
            genesis_block_hash,
            1,
            vec![],
        ));
        let mock_block_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            2,
            0,
            [2; 32],
            [1; 32],
            2,
            vec![],
        ));
        let mock_block_3: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            0,
            [3; 32],
            [2; 32],
            3,
            vec![],
        ));
        let mock_block_4: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            0,
            [4; 32],
            [3; 32],
            4,
            vec![],
        ));
        let mock_block_3_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            0,
            [13; 32],
            [2; 32],
            3,
            vec![],
        ));
        let mock_block_4_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            0,
            [14; 32],
            [13; 32],
            4,
            vec![],
        ));
        let mock_block_5_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            5,
            0,
            [15; 32],
            [14; 32],
            5,
            vec![],
        ));
        let mock_block_6_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            6,
            0,
            [16; 32],
            [15; 32],
            6,
            vec![],
        ));

        longest_chain_queue.roll_forward(mock_block_1.get_hash());
        blocks_database.insert(mock_block_1);
        longest_chain_queue.roll_forward(mock_block_2.get_hash());
        blocks_database.insert(mock_block_2);
        longest_chain_queue.roll_forward(mock_block_3.get_hash());
        blocks_database.insert(mock_block_3);
        longest_chain_queue.roll_forward(mock_block_4.get_hash());
        blocks_database.insert(mock_block_4);
        blocks_database.insert(mock_block_3_2);
        blocks_database.insert(mock_block_4_2);
        blocks_database.insert(mock_block_5_2);
        let blockchain = Blockchain::new(
            fork_manager,
            longest_chain_queue,
            blocks_database,
            block_fee_manager,
            constants.clone(),
            utxoset_ref.clone(),
            mock_mempool_ref.clone(),
            miniblock_mempool_mutex_ref.clone(),
        )
        .await;

        let mut new_chain = vec![];
        blockchain.build_new_chain(&mock_block_6_2, &mut new_chain);
        assert_eq!(new_chain.len(), 4);
        assert_eq!(new_chain[0], [13; 32]);
        assert_eq!(new_chain[1], [14; 32]);
        assert_eq!(new_chain[2], [15; 32]);
        assert_eq!(new_chain[3], [16; 32]);

        let fork_chains = blockchain.find_fork_chains(&mock_block_6_2);
        assert_eq!(fork_chains.new_chain.len(), 4);
        assert_eq!(fork_chains.new_chain[0], [13; 32]);
        assert_eq!(fork_chains.new_chain[1], [14; 32]);
        assert_eq!(fork_chains.new_chain[2], [15; 32]);
        assert_eq!(fork_chains.new_chain[3], [16; 32]);
        assert_eq!(fork_chains.old_chain.len(), 2);
        assert_eq!(fork_chains.old_chain[0], [3; 32]);
        assert_eq!(fork_chains.old_chain[1], [4; 32]);
    }

    #[tokio::test]
    async fn double_spend_on_fork_test() {
        let timestamp_generator = make_timestamp_generator_for_test();

        let constants = Arc::new(Constants::new_for_test(
            None,
            None,
            None,
            None,
            None,
            Some(5),
            None,
        ));

        let keypair = Keypair::new();

        // object under test
        let genesis_block = PandaBlock::new_genesis_block(
            *keypair.get_public_key(),
            timestamp_generator.get_timestamp(),
            1,
        );
        let genesis_block_hash = *genesis_block.get_hash();

        let longest_chain_queue = LongestChainQueue::new(&genesis_block);
        let fork_manager = ForkManager::new(&genesis_block, constants.clone());
        let block_fee_manager = BlockFeeManager::new(constants.clone());
        let blocks_database = BlocksDatabase::new(genesis_block);
        let utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(UtxoSet::new(constants.clone()))));
        let mock_mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(MockMempool::new())));

        let command_line_opts = Arc::new(CommandLineOpts::parse_from(&[
            "pandacoin",
            "--password",
            "asdf",
        ]));
        let keypair_store = Arc::new(KeypairStore::new_mock(command_line_opts));

        let miniblock_mempool_ref = Arc::new(RwLock::new(Box::new(MiniblockMempool::new(
            constants.clone(),
            timestamp_generator.clone(),
            keypair_store.clone(),
            utxoset_ref.clone(),
            mock_mempool_ref.clone(),
        ))));

        let mut blockchain = Blockchain::new(
            fork_manager,
            longest_chain_queue,
            blocks_database,
            block_fee_manager,
            constants.clone(),
            utxoset_ref.clone(),
            mock_mempool_ref.clone(),
            miniblock_mempool_ref.clone(),
        )
        .await;
        //
        //           | e
        //      d2 \ | d
        //       c2 \| c
        //           o b
        //           | a
        //
        // This test will add blocks in this order:
        // a -> b -> c -> c2 -> d2 -> d -> e

        // block_a has a single output in it
        timestamp_generator.advance(10000);
        let output_1 = OutputProto::new(*keypair.get_public_key(), 2);
        let seed_input = OutputIdProto::new([1; 32], 0);
        let tx_1 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![seed_input],
            vec![output_1],
            TxType::Seed,
            vec![],
            keypair.get_secret_key(),
        );
        timestamp_generator.advance(10000);
        let output_1_input = OutputIdProto::new(*tx_1.get_hash(), 0);
        let mock_block_1: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            1,
            1000,
            [1; 32],
            genesis_block_hash,
            timestamp_generator.get_timestamp(),
            vec![tx_1],
        ));

        // block_b spends the output in block_a and creates a new output
        timestamp_generator.advance(10000);
        let output_2 = OutputProto::new(*keypair.get_public_key(), 2);
        let tx_2 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![output_1_input.clone()],
            vec![output_2],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );
        timestamp_generator.advance(10000);
        let output_2_input = OutputIdProto::new(*tx_2.get_hash(), 0);
        let mock_block_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            2,
            1000,
            [2; 32],
            [1; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_2],
        ));

        // block_c spends the output in block_b and creates a new output
        timestamp_generator.advance(10000);
        let output_3 = OutputProto::new(*keypair.get_public_key(), 2);
        let tx_3 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![output_2_input.clone()],
            vec![output_3],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );

        let output_3_2 = OutputProto::new(*keypair.get_public_key(), 1);
        let output_3_2_2 = OutputProto::new(*keypair.get_public_key(), 1);
        let tx_3_2 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![output_2_input.clone()],
            vec![output_3_2, output_3_2_2],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );

        timestamp_generator.advance(10000);
        let output_3_input = OutputIdProto::new(*tx_3.get_hash(), 0);
        let mock_block_3: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            1000,
            [3; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_3],
        ));

        // block_c_2 spends the output in block_b and creates a new output
        timestamp_generator.advance(10000);
        let output_3_2_input = OutputIdProto::new(*tx_3_2.get_hash(), 0);
        let mock_block_3_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            1000,
            [13; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_3_2],
        ));

        // block_d spends the output in block_c and creates a new output
        timestamp_generator.advance(10000);
        let output_4 = OutputProto::new(*keypair.get_public_key(), 2);
        let tx_4 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![output_3_input.clone()],
            vec![output_4],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );

        // block_d_2 spends the output in block_c and creates a new output
        timestamp_generator.advance(10000);
        let output_4_2 = OutputProto::new(*keypair.get_public_key(), 1);
        let tx_4_2 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![output_3_2_input.clone()],
            vec![output_4_2],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );

        timestamp_generator.advance(10000);
        let output_4_input = OutputIdProto::new(*tx_4.get_hash(), 0);
        let mock_block_4: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            1000,
            [4; 32],
            [3; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_4],
        ));

        let _output_4_2_input = OutputIdProto::new(*tx_4_2.get_hash(), 0);
        let mock_block_4_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            1000,
            [14; 32],
            [13; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_4_2],
        ));

        // block_e spends the output in block_d and creates a new output
        timestamp_generator.advance(1000);
        let output_5 = OutputProto::new(*keypair.get_public_key(), 2);
        let tx_5 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![output_4_input.clone()],
            vec![output_5],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );
        let _output_5_input = OutputIdProto::new(*tx_5.get_hash(), 0);
        let mock_block_5: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            5,
            1000,
            [5; 32],
            [4; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_5],
        ));

        // This test will add blocks in this order:
        // a -> b -> c -> c2 -> d2 -> d -> e
        let result: AddBlockEvent = blockchain.add_block(mock_block_1).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);

        let result: AddBlockEvent = blockchain.add_block(mock_block_2).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);

        let result: AddBlockEvent = blockchain.add_block(mock_block_3).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);

        let result: AddBlockEvent = blockchain.add_block(mock_block_3_2).await;
        assert_eq!(result, AddBlockEvent::Accepted);

        let result: AddBlockEvent = blockchain.add_block(mock_block_4_2).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        let result: AddBlockEvent = blockchain.add_block(mock_block_4).await;
        assert_eq!(result, AddBlockEvent::Accepted);

        let result: AddBlockEvent = blockchain.add_block(mock_block_5).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        let mock_block_old: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            0,
            1000,
            [20; 32],
            [0; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));

        let result: AddBlockEvent = blockchain.add_block(mock_block_old).await;
        assert_eq!(result, AddBlockEvent::TooOld);

        // teardown().expect("Teardown failed");
    }
}
