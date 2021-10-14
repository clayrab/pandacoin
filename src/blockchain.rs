use crate::block::RawBlock;
use crate::panda_protos::RawBlockProto;
use crate::forktree::ForkTree;
use crate::longest_chain_queue::{LongestChainQueue};
use crate::types::Sha256Hash;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Initial Treasury
pub const TREASURY: u64 = 286_810_000_000_000_000;

// A lazy-loaded global static reference to Blockchain. For now, we will simply treat
// everything(utxoset, mempool, etc) as a single shared resource which is managed by blockchain.
// In the future we may want to create separate globals for some of the resources being held
// by blockchain by giving them a similar lazy_static Arc<Mutex> treatment, but we will wait
// to see the performance of this simple state management scheme before we try to optimize.
lazy_static! {
    // We use Arc for thread-safe reference counting and Mutex for thread-safe mutabilitity
    pub static ref BLOCKCHAIN_GLOBAL: Arc<RwLock<Blockchain>> = Arc::new(RwLock::new(Blockchain::new()));
}

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

pub struct ForkChains {
    pub ancestor_block: Sha256Hash,
    pub new_chain: Vec<Sha256Hash>,
    pub old_chain: Vec<Sha256Hash>,
}
/// The structure represents the state of the
/// blockchain itself, including the blocks that are on the
/// longest-chain as well as the blocks that is sitting off
/// the longest-chain but capable of being switched over.
#[derive(Debug)]
pub struct Blockchain {
    /// A queue-like structure that holds the longest chain
    longest_chain_queue: LongestChainQueue,
    /// hashmap backed tree to track blocks and potential forks
    fork_tree: ForkTree,
}

impl Blockchain {
    /// Create new `Blockchain`
    pub fn new() -> Self {
        Blockchain {
            longest_chain_queue: LongestChainQueue::new(),
            fork_tree: ForkTree::new(),
        }
    }
    pub fn latest_block(&self) -> Option<&RawBlockProto> {
        //self.fork_tree.block_by_hash(&self.longest_chain_queue.latest_block_hash())
        match self.longest_chain_queue.latest_block_hash() {
            Some(latest_block_hash) => self.fork_tree.block_by_hash(&latest_block_hash),
            None => None,
        }
    }

    /// If the block is in the fork
    pub fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&RawBlockProto> {
        self.fork_tree.block_by_hash(block_hash)
    }

    /// If the block is in the fork
    fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_block_hash(block_hash)
    }

    fn find_fork_chains(&self, block: & dyn RawBlock) -> ForkChains {
        let mut old_chain = vec![];
        let mut new_chain = vec![];

        let mut target_block = block;
        let mut search_completed = false;

        while !search_completed {
            if target_block.get_id() == 0
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
        let ancestor_block = target_block.clone();
        let mut i: u32 = self.longest_chain_queue.latest_block_id();
        // TODO do this in a more rusty way
            
        while i > ancestor_block.get_id() {
            let hash = self.longest_chain_queue.block_hash_by_id(i);
            let block = self.fork_tree.block_by_hash(&hash).unwrap();
            old_chain.push(block.get_hash().clone());
            i = i - 1;
        }
        ForkChains {
            ancestor_block: ancestor_block.get_hash().clone(),
            old_chain: old_chain,
            new_chain: new_chain,
        }
    }

    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    pub async fn add_block(&mut self, block: RawBlockProto) -> AddBlockEvent {
        // TODO: Should we pass a serialized block [u8] to add_block instead of a Block?
        let is_first_block = block.get_previous_block_hash() == [0u8; 32]
            && !self.contains_block_hash(&block.get_previous_block_hash());
        if self.contains_block_hash(&(block.get_hash())) {
            AddBlockEvent::AlreadyKnown
        } else if !is_first_block && !self.contains_block_hash(&block.get_previous_block_hash()) {
            AddBlockEvent::ParentNotFound
        } else {
            
            let fork_chains: ForkChains = self.find_fork_chains(&block);
            if !self.validate_block(&block, &fork_chains) {
                AddBlockEvent::InvalidBlock
            } else {
                let latest_block_hash = self.longest_chain_queue.latest_block_hash();
                let is_new_lc_tip = latest_block_hash == Some(&block.get_previous_block_hash());
                if is_first_block || is_new_lc_tip {
                   
                    // First Block or we'e new tip of the longest chain
                    self.longest_chain_queue.roll_forward(&block.get_hash());
                    // UTXOSET_GLOBAL.clone().write().unwrap().roll_forward(&block);
                    // SLIP_DB_GLOBAL
                    //     .clone()
                    //     .write()
                    //     .unwrap()
                    //     .roll_forward(&block.core());
                
                    let _stored_block = self.fork_tree.insert(block.get_hash().clone(), block).unwrap();
                    

                    // self.storage.roll_forward(&block).await;


                    AddBlockEvent::AcceptedAsLongestChain
                } else {
                    // We are not on the longest chain
                    if self.is_longer_chain(&fork_chains.new_chain, &fork_chains.old_chain) {
                        self.fork_tree.insert(block.get_hash().clone(), block).unwrap();
                        // Unwind the old chain
                        // fork_chains.old_chain.iter().map(|hash| {
                        //     self.longest_chain_queue.roll_back();
                        // });
                        for block_hash in fork_chains.old_chain.iter() {
                            let block: &RawBlockProto = self.fork_tree.block_by_hash(block_hash).unwrap();
                            self.longest_chain_queue.roll_back();
                            // UTXOSET_GLOBAL.clone().write().unwrap().roll_back(block);
                            // SLIP_DB_GLOBAL
                            //     .clone()
                            //     .write()
                            //     .unwrap()
                            //     .roll_back(&block.core());
                            // self.storage.roll_back(&block);
                        }

                        // Wind up the new chain
                        for block_hash in fork_chains.new_chain.iter().rev() {
                            let block: &RawBlockProto = self.fork_tree.block_by_hash(block_hash).unwrap();
                            // self.longest_chain_queue.roll_forward(&block.get_hash());
                            // self.longest_chain_queue.roll_forward(&block.get_hash());
                            // UTXOSET_GLOBAL.clone().write().unwrap().roll_forward(block);
                            // SLIP_DB_GLOBAL
                            //     .clone()
                            //     .write()
                            //     .unwrap()
                            //     .roll_forward(&block.core());
                            // self.storage.roll_forward(block).await;
                        }

                        AddBlockEvent::AcceptedAsNewLongestChain
                    } else {
                        // we're just building on a new chain. Won't take over... yet!
                        // UTXOSET_GLOBAL
                        //     .clone()
                        //     .write()
                        //     .unwrap()
                        //     .roll_forward_on_fork(&block);
                        self.fork_tree.insert(block.get_hash().clone(), block).unwrap();
                        AddBlockEvent::Accepted
                    }
                }
            }
        }
    }

    fn validate_block(&self, block: &RawBlockProto, fork_chains: &ForkChains) -> bool {
        // If the block has an empty hash as previous_block_hash, it's valid no question
        let previous_block_hash = block.get_previous_block_hash();
        if previous_block_hash == [0; 32] && block.get_id() == 0 {
            // if block.burnfee() != DEFAULT_BURN_FEE {
            //     return false;
            // }
            return true;
        } else {
            // If the previous block hash doesn't exist in the ForkTree, it's rejected
            if !self.fork_tree.contains_block_hash(&previous_block_hash) {
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
                return false;
            }

            let previous_block = self.fork_tree.block_by_hash(&previous_block_hash).unwrap();
            
            // TODO validate block fee

            if previous_block.get_timestamp() >= block.get_timestamp() {
                return false;
            }

            // let transactions_valid = block
            //     .transactions()
            //     .par_iter()
            //     .all(|tx| self.validate_transaction(previous_block, tx, fork_chains));

            // transactions_valid
            true
        }
    }

    // fn validate_transaction(
    //     &self,
    //     previous_block: &Block,
    //     tx: &Transaction,
    //     fork_chains: &ForkChains,
    // ) -> bool {
    //     match tx.core.broadcast_type() {
    //         TransactionType::Normal => {
    //             if tx.core.inputs().len() == 0 && tx.core.outputs().len() == 0 {
    //                 return true;
    //             }

    //             if let Some(address) = UTXOSET_GLOBAL
    //                 .clone()
    //                 .read()
    //                 .unwrap()
    //                 .get_receiver_for_inputs(tx.core.inputs())
    //             {
    //                 if !verify_bytes_message(&tx.hash(), &tx.signature(), &address) {
    //                     return false;
    //                 };

    //                 // validate our slips
    //                 let inputs_are_valid = tx.core.inputs().iter().all(|input| {
    //                     if fork_chains.old_chain.len() == 0 {
    //                         return UTXOSET_GLOBAL
    //                             .clone()
    //                             .read()
    //                             .unwrap()
    //                             .is_slip_spendable_at_block(input, previous_block.id());
    //                     } else {
    //                         return UTXOSET_GLOBAL
    //                             .clone()
    //                             .read()
    //                             .unwrap()
    //                             .is_slip_spendable_at_fork_block(input, fork_chains);
    //                     }
    //                 });
    //                 if !inputs_are_valid {
    //                     return false;
    //                 }
    //                 // validate that inputs are unspent
    //                 let input_amt: u64 = tx
    //                     .core
    //                     .inputs()
    //                     .iter()
    //                     .map(|input| {
    //                         UTXOSET_GLOBAL
    //                             .clone()
    //                             .read()
    //                             .unwrap()
    //                             .output_slip_from_slip_id(input)
    //                             .unwrap()
    //                             .amount()
    //                     })
    //                     .sum();

    //                 let output_amt: u64 =
    //                     tx.core.outputs().iter().map(|output| output.amount()).sum();

    //                 if input_amt < output_amt {
    //                     return false;
    //                 }

    //                 return true;
    //             } else {
    //                 // no input slips, thus no output slips.
    //                 return false;
    //             }
    //         }
    //         TransactionType::Seed => {
    //             // need to validate the Seed correctly
    //             return true;
    //         }
    //     }
    // }

    fn is_longer_chain(&self, new_chain: &Vec<Sha256Hash>, old_chain: &Vec<Sha256Hash>) -> bool {
        new_chain.len() > old_chain.len()
    }
}

#[cfg(test)]
mod tests {

}
