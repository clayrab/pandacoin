use async_trait::async_trait;
use futures::StreamExt;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::block::RawBlock;
use crate::crypto::verify_bytes_message;
use crate::forktree::ForkTree;
use crate::longest_chain_queue::LongestChainQueue;
use crate::panda_protos::TransactionProto;
use crate::types::Sha256Hash;
use crate::utxoset::AbstractUtxoSet;

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
}

#[async_trait]
pub trait AbstractBlockchain: Debug {
    fn latest_block(&self) -> Option<&Box<dyn RawBlock>>;

    /// If the block is in the fork
    fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Box<dyn RawBlock>>;
    // fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool;
    // fn find_fork_chains(&self, block: &dyn RawBlock) -> ForkChains;

    async fn add_block(&mut self, block: Box<dyn RawBlock>) -> AddBlockEvent;
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
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
}

#[derive(Debug)]
pub struct Blockchain {
    /// A queue-like structure that holds the longest chain
    longest_chain_queue: LongestChainQueue,
    /// hashmap backed tree to track blocks and potential forks
    fork_tree: ForkTree,
    ///
    context: BlockchainContext,
}

#[async_trait]
impl AbstractBlockchain for Blockchain {
    fn latest_block(&self) -> Option<&Box<dyn RawBlock>> {
        //self.fork_tree.block_by_hash(&self.longest_chain_queue.latest_block_hash())
        match self.longest_chain_queue.latest_block_hash() {
            Some(latest_block_hash) => self.fork_tree.block_by_hash(&latest_block_hash),
            None => None,
        }
    }
    /// If the block is in the fork
    fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Box<dyn RawBlock>> {
        self.fork_tree.block_by_hash(block_hash)
    }
    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    //async fn add_block(&mut self, block: RawBlockProto) -> AddBlockEvent {
    async fn add_block(&mut self, block: Box<dyn RawBlock>) -> AddBlockEvent {
        // TODO: Should we pass a serialized block [u8] to add_block instead of a Block?
        let is_first_block = block.get_previous_block_hash() == [0u8; 32]
            && !self.contains_block_hash(&block.get_previous_block_hash());
        if self.contains_block_hash(&(block.get_hash())) {
            AddBlockEvent::AlreadyKnown
        } else if !is_first_block && !self.contains_block_hash(&block.get_previous_block_hash()) {
            AddBlockEvent::ParentNotFound
        } else {
            let fork_chains: ForkChains = self.find_fork_chains(&block);
            if !self.validate_block(&block, &fork_chains).await {
                AddBlockEvent::InvalidBlock
            } else {
                let latest_block_hash = self.longest_chain_queue.latest_block_hash();
                let is_new_lc_tip = latest_block_hash == Some(&block.get_previous_block_hash());
                if is_first_block || is_new_lc_tip {
                    // First Block or we'e new tip of the longest chain
                    self.longest_chain_queue.roll_forward(&block.get_hash());
                    let mut utxoset = self.context.utxoset_ref.write().await;
                    utxoset.roll_forward(&block);
                    // OUTPUT_DB_GLOBAL
                    //     .clone()
                    //     .write()
                    //     .unwrap()
                    //     .roll_forward(&block.core());

                    let _stored_block = self
                        .fork_tree
                        .insert(block.get_hash().clone(), block)
                        .unwrap();

                    // self.storage.roll_forward(&block).await;

                    AddBlockEvent::AcceptedAsLongestChain
                } else {
                    // We are not on the longest chain
                    if self.is_longer_chain(&fork_chains.new_chain, &fork_chains.old_chain) {
                        self.fork_tree
                            .insert(block.get_hash().clone(), block)
                            .unwrap();
                        // Unwind the old chain
                        let _result = fork_chains.old_chain.iter().map(|_hash| {
                            self.longest_chain_queue.roll_back();
                        });
                        for block_hash in fork_chains.old_chain.iter() {
                            let block: &Box<dyn RawBlock> =
                                self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_back();
                            let mut utxoset = self.context.utxoset_ref.write().await;
                            utxoset.roll_back(&block);
                            // OUTPUT_DB_GLOBAL
                            //     .clone()
                            //     .write()
                            //     .unwrap()
                            //     .roll_back(&block.core());
                            // self.storage.roll_back(&block);
                        }

                        // Wind up the new chain
                        for block_hash in fork_chains.new_chain.iter().rev() {
                            let block: &Box<dyn RawBlock> =
                                self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_forward(&block.get_hash());
                            let mut utxoset = self.context.utxoset_ref.write().await;
                            utxoset.roll_forward(&block);
                            // OUTPUT_DB_GLOBAL
                            //     .clone()
                            //     .write()
                            //     .unwrap()
                            //     .roll_forward(&block.core());
                            // self.storage.roll_forward(block).await;
                        }
                        // Wind the old chain as a fork chain
                        for block_hash in fork_chains.old_chain.iter() {
                            let block: &Box<dyn RawBlock> =
                                self.fork_tree.block_by_hash(block_hash).unwrap();
                            let mut utxoset = self.context.utxoset_ref.write().await;
                            utxoset.roll_forward_on_fork(&block);
                            // roll_forward_on_fork
                            //     .clone()
                            //     .write()
                            //     .unwrap()
                            //     .roll_back(&block.core());
                            // self.storage.roll_back(&block);
                        }

                        AddBlockEvent::AcceptedAsNewLongestChain
                    } else {
                        // we're just building on a new chain. Won't take over... yet!
                        let mut utxoset = self.context.utxoset_ref.write().await;
                        utxoset.roll_forward_on_fork(&block);

                        self.fork_tree
                            .insert(block.get_hash().clone(), block)
                            .unwrap();
                        AddBlockEvent::Accepted
                    }
                }
            }
        }
    }
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new(utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>) -> Self {
        // let context = BlockchainContext {
        //     utxoset_ref
        // }
        Blockchain {
            longest_chain_queue: LongestChainQueue::new(),
            fork_tree: ForkTree::new(),
            context: BlockchainContext { utxoset_ref },
        }
    }

    /// If the block is in the fork
    fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_block_hash(block_hash)
    }

    fn find_fork_chains(&self, block: &Box<dyn RawBlock>) -> ForkChains {
        let mut old_chain = vec![];
        let mut new_chain = vec![];

        let mut target_block = block;
        let mut search_completed = false;

        while !search_completed {
            if target_block.get_id() == 1
                || self
                    .longest_chain_queue
                    .contains_hash_by_block_id(&target_block.get_hash(), target_block.get_id())
            {
                search_completed = true;
            } else {
                new_chain.push(target_block.get_hash().clone());
                match self
                    .fork_tree
                    .block_by_hash(&target_block.get_previous_block_hash())
                {
                    Some(previous_block) => target_block = previous_block,
                    None => {
                        search_completed = true;
                    }
                }
            }
        }

        // TODO Can we remove this clone??
        //let ancestor_block = target_block.clone();
        let mut i: u32 = self.longest_chain_queue.latest_block_id();
        // TODO do this in a more rusty way
        while i > target_block.get_id() {
            let hash = self.longest_chain_queue.block_hash_by_id(i).unwrap();
            let block = self.fork_tree.block_by_hash(&hash).unwrap();
            old_chain.push(block.get_hash().clone());
            i = i - 1;
        }
        ForkChains {
            ancestor_block_hash: target_block.get_hash().clone(),
            ancestor_block_id: target_block.get_id(),
            old_chain: old_chain,
            new_chain: new_chain,
        }
    }

    async fn validate_block(&self, block: &Box<dyn RawBlock>, fork_chains: &ForkChains) -> bool {
        // If the block has an empty hash as previous_block_hash, it's valid no question
        let previous_block_hash = block.get_previous_block_hash();
        if previous_block_hash == [0; 32] && block.get_id() == 1 {
            // if block.burnfee() != DEFAULT_BURN_FEE {
            //     return false;
            // }
            return true;
        } else {
            // If the previous block hash doesn't exist in the ForkTree, it's rejected
            if !self.fork_tree.contains_block_hash(&previous_block_hash) {
                info!("invalid block, previous block not found");
                return false;
            }
            if self
                .fork_tree
                .block_by_hash(&previous_block_hash)
                .unwrap()
                .get_id()
                + 1
                != block.get_id()
            {
                info!("invalid block, wrong block id");
                return false;
            }

            let previous_block = self.fork_tree.block_by_hash(&previous_block_hash).unwrap();

            // TODO validate block fee

            if previous_block.get_timestamp() >= block.get_timestamp() {
                info!("invalid block, timestamp must be greater than previous block");
                return false;
            }

            //let transactions_iterator_stream = futures::stream::iter(block.get_transactions());
            // while let Some(transaction) = transactions_iterator_stream.next().await {
            //     self.validate_transaction(previous_block, transaction, fork_chains).await;
            //     // if peer.get_has_completed_handshake() {
            //     //     let send_block_head_message = SendBlockHeadMessage::new(block_hash);
            //     //     peer.send_command_fire_and_forget("SNDBLKHD", send_block_head_message.serialize())
            //     //         .await;
            //     // } else {
            //     //     info!("Hasn't completed handshake, will not send block??");
            //     // }
            // }
            futures::stream::iter(block.get_transactions())
                .all(|transaction| {
                    self.validate_transaction(previous_block, transaction, fork_chains)
                })
                .await

            // let transactions_valid = block
            //     .get_transactions()
            //     .par_iter()
            //     .all(|tx| self.validate_transaction(previous_block, tx, fork_chains).await);

            // transactions_valid
            //true
        }
    }

    async fn validate_transaction(
        &self,
        previous_block: &Box<dyn RawBlock>,
        tx: &TransactionProto,
        fork_chains: &ForkChains,
    ) -> bool {
        match tx.txtype {
            //TxType::Normal => {
            0 => {
                if tx.inputs.len() == 0 && tx.outputs.len() == 0 {
                    return true;
                }
                let utxoset = self.context.utxoset_ref.read().await;
                if let Some(address) = utxoset.get_receiver_for_inputs(&tx.inputs) {
                    if !verify_bytes_message(&tx.hash(), tx.signature.clone(), &address) {
                        info!("tx signature invalid");
                        return false;
                    };

                    // validate our outputs
                    // TODO: remove this clone?
                    let inputs_iterator_stream = futures::stream::iter(&tx.inputs);
                    let inputs_are_valid = inputs_iterator_stream
                        .all(|input| {
                            if fork_chains.old_chain.len() == 0 {
                                info!("is_output_spendable_at_block_id");
                                utxoset
                                    .is_output_spendable_at_block_id(input, previous_block.get_id())
                            } else {
                                info!("is_output_spendable_in_fork_branch");
                                utxoset.is_output_spendable_in_fork_branch(input, fork_chains)
                            }
                        })
                        .await;

                    if !inputs_are_valid {
                        info!("tx invalid inputs");
                        return false;
                    }
                    // validate that inputs are unspent
                    let input_amt: u64 = tx
                        .inputs
                        .iter()
                        .map(|input| {
                            utxoset
                                .output_status_from_output_id(input)
                                .unwrap()
                                .amount()
                        })
                        .sum();

                    let output_amt: u64 = tx.outputs.iter().map(|output| output.amount()).sum();

                    let is_balanced = output_amt == input_amt;
                    if !is_balanced {
                        info!("inputs/outputs not balanced");
                    }
                    is_balanced
                } else {
                    info!("no single receiver for inputs");
                    false
                }
            }
            1 => {
                // need to validate the Seed correctly
                true
            }
            _ => {
                error!("Unknown Transaction Type");
                false
            }
        }
    }

    fn is_longer_chain(&self, new_chain: &Vec<Sha256Hash>, old_chain: &Vec<Sha256Hash>) -> bool {
        new_chain.len() > old_chain.len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::RwLock;

    use crate::utxoset::AbstractUtxoSet;
    use crate::{
        block::RawBlock,
        blockchain::Blockchain,
        keypair::Keypair,
        panda_protos::{transaction_proto::TxType, OutputIdProto, OutputProto, TransactionProto},
        test_utilities::{
            globals_init::make_timestamp_generator_for_test, mock_block::MockRawBlockForBlockchain,
        },
        utxoset::UtxoSet,
    };

    use super::{AbstractBlockchain, AddBlockEvent};

    fn teardown() -> std::io::Result<()> {
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
    async fn double_spend_on_fork_test() {
        let timestamp_generator = make_timestamp_generator_for_test();
        let utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(UtxoSet::new())));
        let keypair = Keypair::new();
        // object under test
        let mut blockchain = Blockchain::new(utxoset_ref.clone());

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
        let output_a = OutputProto::new(keypair.get_public_key().clone(), 2);
        let tx_a = TransactionProto::new(
            vec![],
            vec![output_a],
            TxType::Normal,
            timestamp_generator.get_timestamp(),
            vec![],
        );
        let output_a_input = OutputIdProto::new(tx_a.get_hash(), 0);
        let mock_block_a: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            1,
            [1; 32],
            [0; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_a],
        ));

        // block_b spends the output in block_a and creates a new output
        timestamp_generator.advance(1000);
        let output_b = OutputProto::new(keypair.get_public_key().clone(), 2);
        let tx_b = TransactionProto::new(
            vec![output_a_input.clone()],
            vec![output_b],
            TxType::Seed,
            timestamp_generator.get_timestamp(),
            vec![],
        );
        let output_b_input = OutputIdProto::new(tx_b.get_hash(), 0);
        let mock_block_b: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            2,
            [2; 32],
            [1; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_b],
        ));

        // block_c spends the output in block_b and creates a new output
        timestamp_generator.advance(1000);
        let output_c = OutputProto::new(keypair.get_public_key().clone(), 2);
        let tx_c = TransactionProto::new(
            vec![output_b_input.clone()],
            vec![output_c],
            TxType::Normal,
            timestamp_generator.get_timestamp(),
            vec![],
        );
        let output_c_input = OutputIdProto::new(tx_c.get_hash(), 0);
        let mock_block_c: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            [3; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_c],
        ));

        // block_c_2 spends the output in block_b and creates a new output
        timestamp_generator.advance(1);
        let output_c_2 = OutputProto::new(keypair.get_public_key().clone(), 1);
        let output_c_2_2 = OutputProto::new(keypair.get_public_key().clone(), 1);
        let tx_c_2 = TransactionProto::new(
            vec![output_b_input.clone()],
            vec![output_c_2, output_c_2_2],
            TxType::Normal,
            timestamp_generator.get_timestamp(),
            vec![],
        );
        let output_c_2_input = OutputIdProto::new(tx_c_2.get_hash(), 0);
        let mock_block_c_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            [6; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_c_2],
        ));

        // block_d spends the output in block_c and creates a new output
        timestamp_generator.advance(1000);
        let output_d = OutputProto::new(keypair.get_public_key().clone(), 2);
        let tx_d = TransactionProto::new(
            vec![output_c_input.clone()],
            vec![output_d],
            TxType::Normal,
            timestamp_generator.get_timestamp(),
            vec![],
        );
        let output_d_input = OutputIdProto::new(tx_d.get_hash(), 0);
        let mock_block_d: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            [4; 32],
            [3; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_d],
        ));

        // block_d_2 spends the output in block_c and creates a new output
        timestamp_generator.advance(1000);
        let output_d_2 = OutputProto::new(keypair.get_public_key().clone(), 1);
        let tx_d_2 = TransactionProto::new(
            vec![output_c_2_input.clone()],
            vec![output_d_2],
            TxType::Normal,
            timestamp_generator.get_timestamp(),
            vec![],
        );
        let _output_d_2_input = OutputIdProto::new(tx_d_2.get_hash(), 0);
        let mock_block_d_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            [7; 32],
            [6; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_d_2],
        ));

        // block_e spends the output in block_d and creates a new output
        timestamp_generator.advance(1000);
        let output_e = OutputProto::new(keypair.get_public_key().clone(), 2);
        let tx_e = TransactionProto::new(
            vec![output_d_input.clone()],
            vec![output_e],
            TxType::Normal,
            timestamp_generator.get_timestamp(),
            vec![],
        );
        let _output_e_input = OutputIdProto::new(tx_e.get_hash(), 0);
        let mock_block_e: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            5,
            [5; 32],
            [4; 32],
            timestamp_generator.get_timestamp(),
            vec![tx_e],
        ));

        let result: AddBlockEvent = blockchain.add_block(mock_block_a).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);

        let result: AddBlockEvent = blockchain.add_block(mock_block_b).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);

        let result: AddBlockEvent = blockchain.add_block(mock_block_c).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);

        // This test will add blocks in this order:
        // a -> b -> c -> c2 -> d2 -> d -> e
        let result: AddBlockEvent = blockchain.add_block(mock_block_c_2).await;
        assert_eq!(result, AddBlockEvent::Accepted);

        let result: AddBlockEvent = blockchain.add_block(mock_block_d_2).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

        let result: AddBlockEvent = blockchain.add_block(mock_block_d).await;
        assert_eq!(result, AddBlockEvent::Accepted);

        let result: AddBlockEvent = blockchain.add_block(mock_block_e).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);
    }

    // #[tokio::test]
    // async fn add_block_test() {
    //     let keypair = Keypair::new();
    //     let (mut blockchain, _slips) =
    //         test_utilities::mocks::make_mock_blockchain_and_slips(&keypair, 3 * EPOCH_LENGTH).await;
    //     let mut mock_timestamp_generator = MockTimestampGenerator::new();
    //     let block = blockchain.latest_block().unwrap().clone();
    //     let mut prev_block_hash = block.hash();
    //     let mut prev_block_id = block.id();
    //     let mut prev_timestamp = block.timestamp();
    //     let mut prev_burn_fee = block.burnfee();
    //     let mut prev_prev_burn_fee = block.burnfee();
    //     let first_block_hash = block.hash();
    //     let first_burn_fee = block.burnfee();
    //     let first_timestamp = block.timestamp();

    //     for n in 0..5 as i32 {
    //         let timestamp = mock_timestamp_generator.next();
    //         let block = Block::new(BlockCore::new(
    //             prev_block_id + 1,
    //             timestamp,
    //             prev_block_hash,
    //             *keypair.public_key(),
    //             TREASURY,
    //             BurnFee::burn_fee_adjustment_calculation(
    //                 prev_burn_fee,
    //                 prev_prev_burn_fee,
    //                 timestamp,
    //                 prev_timestamp,
    //             ),
    //             vec![],
    //         ));
    //         let result: AddBlockEvent = blockchain.add_block(block.clone()).await;

    //         assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
    //         assert_eq!(blockchain.latest_block().unwrap().id(), (n + 1) as u64);
    //         prev_block_hash = block.hash().clone();
    //         prev_block_id = block.id();
    //         prev_timestamp = block.timestamp();
    //         prev_burn_fee = block.burnfee();
    //         prev_prev_burn_fee = blockchain
    //             .get_block_by_hash(block.previous_block_hash())
    //             .unwrap()
    //             .burnfee();

    //         // println!("{:?}", result);
    //     }
    //     // make a fork
    //     let timestamp = mock_timestamp_generator.next();
    //     let block = Block::new(BlockCore::new(
    //         1,
    //         timestamp,
    //         first_block_hash,
    //         *keypair.public_key(),
    //         TREASURY,
    //         BurnFee::burn_fee_adjustment_calculation(
    //             first_burn_fee,
    //             first_burn_fee,
    //             timestamp,
    //             first_timestamp,
    //         ),
    //         vec![],
    //     ));

    //     prev_block_hash = block.hash().clone();
    //     prev_block_id = block.id();
    //     prev_timestamp = block.timestamp();
    //     prev_burn_fee = block.burnfee();
    //     prev_prev_burn_fee = blockchain
    //         .get_block_by_hash(block.previous_block_hash())
    //         .unwrap()
    //         .burnfee();

    //     let result: AddBlockEvent = blockchain.add_block(block.clone()).await;

    //     // println!("{:?}", result);
    //     assert_eq!(result, AddBlockEvent::Accepted);

    //     for _ in 0..4 as i32 {
    //         let timestamp = mock_timestamp_generator.next();
    //         let block = Block::new(BlockCore::new(
    //             prev_block_id + 1,
    //             timestamp,
    //             prev_block_hash,
    //             *keypair.public_key(),
    //             TREASURY,
    //             BurnFee::burn_fee_adjustment_calculation(
    //                 prev_burn_fee,
    //                 prev_prev_burn_fee,
    //                 timestamp,
    //                 prev_timestamp,
    //             ),
    //             vec![],
    //         ));
    //         let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
    //         assert_eq!(result, AddBlockEvent::Accepted);
    //         prev_block_hash = block.hash().clone();
    //         prev_block_id = block.id();
    //         prev_timestamp = block.timestamp();
    //         prev_burn_fee = block.burnfee();
    //         prev_prev_burn_fee = blockchain
    //             .get_block_by_hash(block.previous_block_hash())
    //             .unwrap()
    //             .burnfee();
    //     }
    //     // new longest chain tip on top of the fork
    //     let timestamp = mock_timestamp_generator.next();
    //     let block = Block::new(BlockCore::new(
    //         prev_block_id + 1,
    //         timestamp,
    //         prev_block_hash,
    //         *keypair.public_key(),
    //         TREASURY,
    //         BurnFee::burn_fee_adjustment_calculation(
    //             prev_burn_fee,
    //             prev_prev_burn_fee,
    //             timestamp,
    //             prev_timestamp,
    //         ),
    //         vec![],
    //     ));

    //     let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
    //     // println!("{:?}", result);
    //     assert_eq!(blockchain.latest_block().unwrap().hash(), block.hash());
    //     assert_eq!(blockchain.latest_block().unwrap().id(), block.id());

    //     assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

    //     // make another fork
    //     let timestamp = mock_timestamp_generator.next();
    //     let block = Block::new(BlockCore::new(
    //         1,
    //         timestamp,
    //         first_block_hash,
    //         *keypair.public_key(),
    //         TREASURY,
    //         BurnFee::burn_fee_adjustment_calculation(
    //             first_burn_fee,
    //             first_burn_fee,
    //             timestamp,
    //             first_timestamp,
    //         ),
    //         vec![],
    //     ));
    //     let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
    //     // println!("{:?}", result);
    //     assert_eq!(result, AddBlockEvent::Accepted);

    //     prev_block_hash = block.hash().clone();
    //     prev_block_id = block.id();
    //     prev_timestamp = block.timestamp();
    //     prev_burn_fee = block.burnfee();
    //     prev_prev_burn_fee = blockchain
    //         .get_block_by_hash(block.previous_block_hash())
    //         .unwrap()
    //         .burnfee();
    //     for _n in 0..5 as i32 {
    //         let timestamp = mock_timestamp_generator.next();
    //         let block = Block::new(BlockCore::new(
    //             prev_block_id + 1,
    //             timestamp,
    //             prev_block_hash,
    //             *keypair.public_key(),
    //             TREASURY,
    //             BurnFee::burn_fee_adjustment_calculation(
    //                 prev_burn_fee,
    //                 prev_prev_burn_fee,
    //                 timestamp,
    //                 prev_timestamp,
    //             ),
    //             vec![],
    //         ));
    //         let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
    //         // println!("{:?}", result);
    //         assert_eq!(result, AddBlockEvent::Accepted);

    //         prev_block_hash = block.hash().clone();
    //         prev_block_id = block.id();
    //         prev_timestamp = block.timestamp();
    //         prev_burn_fee = block.burnfee();
    //         prev_prev_burn_fee = blockchain
    //             .get_block_by_hash(block.previous_block_hash())
    //             .unwrap()
    //             .burnfee();
    //     }
    //     // new longest chain tip on top of the fork
    //     let timestamp = mock_timestamp_generator.next();
    //     let block = Block::new(BlockCore::new(
    //         prev_block_id + 1,
    //         timestamp,
    //         prev_block_hash,
    //         *keypair.public_key(),
    //         TREASURY,
    //         BurnFee::burn_fee_adjustment_calculation(
    //             prev_burn_fee,
    //             prev_prev_burn_fee,
    //             timestamp,
    //             prev_timestamp,
    //         ),
    //         vec![],
    //     ));
    //     let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
    //     // println!("{:?}", result);
    //     assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

    //     // make a 3rd fork by rolling back by 3 blocks
    //     prev_block_hash = blockchain
    //         .latest_block()
    //         .unwrap()
    //         .previous_block_hash()
    //         .clone();
    //     let mut prev_block = blockchain.get_block_by_hash(&prev_block_hash).unwrap();
    //     prev_block_hash = prev_block.previous_block_hash().clone();
    //     prev_block = blockchain.get_block_by_hash(&prev_block_hash).unwrap();

    //     prev_block_hash = prev_block.hash().clone();
    //     prev_block_id = prev_block.id();
    //     prev_timestamp = prev_block.timestamp();
    //     prev_burn_fee = prev_block.burnfee();
    //     prev_prev_burn_fee = blockchain
    //         .get_block_by_hash(prev_block.previous_block_hash())
    //         .unwrap()
    //         .burnfee();

    //     for _ in 0..2 as i32 {
    //         let timestamp = mock_timestamp_generator.next();
    //         let block = Block::new(BlockCore::new(
    //             prev_block_id + 1,
    //             timestamp,
    //             prev_block_hash,
    //             *keypair.public_key(),
    //             TREASURY,
    //             BurnFee::burn_fee_adjustment_calculation(
    //                 prev_burn_fee,
    //                 prev_prev_burn_fee,
    //                 timestamp,
    //                 prev_timestamp,
    //             ),
    //             vec![],
    //         ));
    //         let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
    //         // println!("{:?}", result);
    //         assert_eq!(result, AddBlockEvent::Accepted);

    //         prev_block_hash = block.hash().clone();
    //         prev_block_id = block.id();
    //         prev_timestamp = block.timestamp();
    //         prev_burn_fee = block.burnfee();
    //         prev_prev_burn_fee = blockchain
    //             .get_block_by_hash(block.previous_block_hash())
    //             .unwrap()
    //             .burnfee();
    //     }

    //     let timestamp = mock_timestamp_generator.next();
    //     let block = Block::new(BlockCore::new(
    //         prev_block_id + 1,
    //         timestamp,
    //         prev_block_hash,
    //         *keypair.public_key(),
    //         TREASURY,
    //         BurnFee::burn_fee_adjustment_calculation(
    //             prev_burn_fee,
    //             prev_prev_burn_fee,
    //             timestamp,
    //             prev_timestamp,
    //         ),
    //         vec![],
    //     ));
    //     let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
    //     // println!("{:?}", result);
    //     assert_eq!(result, AddBlockEvent::AcceptedAsNewLongestChain);

    //     // make another fork
    //     let timestamp = mock_timestamp_generator.next();
    //     let block = Block::new(BlockCore::new(
    //         1,
    //         timestamp,
    //         first_block_hash,
    //         *keypair.public_key(),
    //         TREASURY,
    //         BurnFee::burn_fee_adjustment_calculation(
    //             first_burn_fee,
    //             first_burn_fee,
    //             timestamp,
    //             first_timestamp,
    //         ),
    //         vec![],
    //     ));
    //     let result: AddBlockEvent = blockchain.add_block(block.clone()).await;
    //     // println!("{:?}", result);
    //     assert_eq!(result, AddBlockEvent::Accepted);

    //     // TODO repair this last test. Running the burnfee past 2 epochs causes an integer overflow...

    //     // prev_block_hash = block.hash().clone();
    //     // prev_block_id = block.id();
    //     // prev_timestamp = block.timestamp();
    //     // prev_burn_fee = block.burnfee();
    //     // prev_prev_burn_fee = blockchain.get_block_by_hash(block.previous_block_hash()).unwrap().burnfee();

    //     // run past 2x EPOCH_LENGTH blocks to test the epoch a bit
    //     // for _n in 0..(2 * EPOCH_LENGTH + 1) {
    //     //     let timestamp = mock_timestamp_generator.next();
    //     //     let block = Block::new(BlockCore::new(
    //     //         prev_block_id + 1,
    //     //         timestamp,
    //     //         prev_block_hash,
    //     //         *keypair.public_key(),
    //     //         0,
    //     //         TREASURY,
    //     //         BurnFee::burn_fee_adjustment_calculation(
    //     //             prev_burn_fee,
    //     //             prev_prev_burn_fee,
    //     //             timestamp,
    //     //             prev_timestamp
    //     //         ),
    //     //         vec![],
    //     //     ));
    //     //     let result = blockchain.add_block(block.clone()).await;

    //     //     // weak assertion is okay here, we just want to make sure nothing panics
    //     //     assert!(
    //     //         result == AddBlockEvent::Accepted
    //     //             || result == AddBlockEvent::AcceptedAsLongestChain
    //     //             || result == AddBlockEvent::AcceptedAsNewLongestChain
    //     //     );

    //     //     prev_block_hash = block.hash().clone();
    //     //     prev_block_id = block.id();
    //     //     prev_timestamp = block.timestamp();
    //     //     prev_burn_fee = block.burnfee();
    //     //     prev_prev_burn_fee = blockchain.get_block_by_hash(block.previous_block_hash()).unwrap().burnfee();
    //     // }

    //     teardown().expect("Teardown failed");
    // }
}
