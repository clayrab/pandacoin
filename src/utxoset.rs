use dashmap::mapref::entry::Entry;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::block::RawBlock;
use crate::blockchain::ForkChains;
use crate::panda_protos::{OutputIdProto, OutputProto, TransactionProto};
use crate::types::Sha256Hash;

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

lazy_static! {

    // We use Arc for thread-safe reference counting and Mutex for thread-safe mutabilitity
    pub static ref UTXOSET_GLOBAL: Arc<std::sync::RwLock<UtxoSet>> = Arc::new(std::sync::RwLock::new(UtxoSet::new()));
}

#[derive(Debug, Clone, PartialEq)]
enum LongestChainSpentTime {
    BeforeUnspent,
    BetweenUnspentAndSpent,
    AfterSpent,
    NeverExisted,
    AfterSpentOrNeverExisted,
}

/// The fork spent status indicates when a output on a fork block was
/// unspent or spent. It is stored in a map keyed by the block hash
/// where the event occurred. This could be optimized to use the
/// block id. Block hash is 32 bytes, block id is 8, however, this
/// would also require a way to keep track of forks.
#[derive(Debug, Clone, Hash, PartialEq)]
enum ForkSpentStatus {
    ForkUnspent,
    ForkSpent,
}

//           |
//       \   x 
//        x  |    /
//         \ |   /
//          \|  x
//           o o
//           |/
//           |
//
// o = output, x = spent

/// This structure is stored in the values of the utxoset map.
/// This struct serves two purposes: to validate spendability on the 
/// longest chain and to validate spendability on forks.
/// For forks, we simply keep a status for each block_hash where the
/// output has been unspent or spent, which can occur in multiple
/// forks.
/// 
/// The longest chain status is stored more efficiently as two Option<u32>, i.e. the 
/// block ids where the output is created and subsequently spent.
/// 
/// The fork_statuses map stores spent/unspent(created) enums for each block hash where
/// the output appears in a fork. This can be optimized later.
/// 
#[derive(Debug, Clone, PartialEq)]
struct SlipSpentStatus {
    output_output: OutputProto,
    longest_chain_unspent_block_id: Option<u32>,
    longest_chain_spent_block_id: Option<u32>,
    fork_statuses: HashMap<Sha256Hash, ForkSpentStatus>,
}
impl SlipSpentStatus {
    /// When we create a new SlipSpentStatus, this is because it is the first
    /// time we have seen this output. If this is on the longest chain we can
    /// use this constructor.
    pub fn new_on_longest_chain(output_output: OutputProto, unspent_block_id: u32) -> Self {
        SlipSpentStatus {
            output_output: output_output,
            longest_chain_unspent_block_id: Some(unspent_block_id),
            longest_chain_spent_block_id: None,
            fork_statuses: HashMap::new(),
        }
    }

    /// When we create a new SlipSpentStatus, this is because it is the first
    /// time we have seen this output. If this is in a fork block we can
    /// use this constructor.
    pub fn new_on_fork(output_output: OutputProto, block_hash: Sha256Hash) -> Self {
        let mut fork_statuses_map: HashMap<Sha256Hash, ForkSpentStatus> = HashMap::new();
        fork_statuses_map.insert(block_hash, ForkSpentStatus::ForkUnspent);
        SlipSpentStatus {
            output_output: output_output,
            longest_chain_unspent_block_id: None,
            longest_chain_spent_block_id: None,
            fork_statuses: fork_statuses_map,
        }
    }
}

/// A hashmap storing everything needed to validate the spendability of a output.
/// This may be optimized in the future, but should be performant enough for the
/// time being.
#[derive(Debug, Clone)]
pub struct UtxoSet {
    status_map: DashMap<OutputIdProto, SlipSpentStatus>,
}

impl UtxoSet {
    /// Create new `UtxoSet`
    pub fn new() -> Self {
        UtxoSet {
            status_map: DashMap::new(),
        }
    }
    /// Removes a block from the tip of a fork chain. This is not technically needed yet,
    /// but might be very helpful if we wanted to cleanup a fork, especially if it is a
    /// fork of a fork.
    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Inputs can just be removed(delete the appropriate ForkSpent from the vector, the
    /// ForkUnspent is still in the vector). Outputs should also have their ForkSpent removed from
    /// the vector.

    pub fn roll_back_on_fork(&mut self, block: &Box<dyn RawBlock>) {
        block.get_transactions().par_iter().for_each(|tx| {
            tx
                .outputs
                .par_iter()
                .enumerate()
                .for_each(|(index, _output)| {
                    let output_id = OutputIdProto::new(tx.get_hash(), index as u32);
                    let entry = self
                        .status_map
                        .entry(output_id)
                        .and_modify(|output_spent_status| {
                            output_spent_status.fork_statuses.remove(&block.get_hash());
                        });
                    if let Entry::Vacant(_o) = entry {
                        panic!("Output fork status not found in hashmap!");
                    }
                });
            tx
                .inputs
                .par_iter()
                .enumerate()
                .for_each(|(_index, input)| {
                    let entry =
                        self.status_map
                            .entry(input.clone())
                            .and_modify(|output_spent_status| {
                                output_spent_status.fork_statuses.remove(&block.get_hash());
                            });
                    if let Entry::Vacant(_o) = entry {
                        panic!("Input fork status not found in hashmap!");
                    }
                });
        });
    }
    /// Add a block to the tip of a fork.
    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Outputs should be added or marked as ForkUnspent, Inputs should be marked ForkSpent.
    /// This method be called when the block is first seen but should never need to be called
    /// during a reorg.
    pub fn roll_forward_on_fork(&mut self, block: &Box<dyn RawBlock>) {
        block.get_transactions().par_iter().for_each(|tx| {
            tx
                .outputs
                .iter()
                .enumerate()
                .for_each(|(index, output)| {
                    let output_id = OutputIdProto::new(tx.get_hash(), index as u32);
                    self.status_map
                        .entry(output_id)
                        .and_modify(|output_spent_status: &mut SlipSpentStatus| {
                            output_spent_status
                                .fork_statuses
                                .insert(block.get_hash(), ForkSpentStatus::ForkUnspent);
                        })
                        .or_insert(SlipSpentStatus::new_on_fork(output.clone(), block.get_hash()));
                });
            // loop through inputs and mark them as ForkSpent
            tx.inputs.iter().for_each(|input| {
                self.status_map.entry(input.clone()).and_modify(
                    |output_spent_status: &mut SlipSpentStatus| {
                        output_spent_status
                            .fork_statuses
                            .insert(block.get_hash(), ForkSpentStatus::ForkSpent);
                    },
                );
            });
        });
    }
    /// Remove a block from the tip of the longest chain.
    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Inputs should be marked back to Unspent, Outputs should have all status set to None. We
    /// do not delete Outputs from the hashmap because they will soon be "unspent" again when
    /// the transaction is rolled forward in another block.
    pub fn roll_back(&mut self, block: &Box<dyn RawBlock>) {
        // unspend outputs and spend the inputs
        block.get_transactions().par_iter().for_each(|tx| {
            tx
                .outputs
                .par_iter()
                .enumerate()
                .for_each(|(index, _output)| {
                    let output_id = OutputIdProto::new(tx.get_hash(), index as u32);
                    let entry = self
                        .status_map
                        .entry(output_id)
                        .and_modify(|output_spent_status| {
                            output_spent_status.longest_chain_unspent_block_id = None;
                        });
                    if let Entry::Vacant(_o) = entry {
                        panic!("Output status not found in hashmap!");
                    }
                });
            tx
                .inputs
                .par_iter()
                .enumerate()
                .for_each(|(_index, input)| {
                    let entry =
                        self.status_map
                            .entry(input.clone())
                            .and_modify(|output_spent_status| {
                                output_spent_status.longest_chain_spent_block_id = None;
                            });
                    if let Entry::Vacant(_o) = entry {
                        panic!("Input status not found in hashmap!");
                    }
                });
        });
    }
    /// Add a block to the tip of the longest chain.
    /// Loop through the inputs and outputs in a transaction update the hashmap appropriately.
    /// Outputs should be added or marked Unspent, Inputs should be marked Spent. This method
    /// Can be called during a normal new block or during a reorg, so Unspent Outputs may already
    /// be present if we're doing a reorg.
    //pub fn roll_forward(&mut self, block: &dyn RawBlock) {
    pub fn roll_forward(&mut self, block: &Box<dyn RawBlock>) {
        block.get_transactions().par_iter().for_each(|tx| {
            tx
                .outputs
                .par_iter()
                .enumerate()
                .for_each(|(index, output)| {
                    let output_id = OutputIdProto::new(tx.get_hash(), index as u32);
                    self.status_map
                        .entry(output_id)
                        .and_modify(|output_spent_status| {
                            output_spent_status.longest_chain_spent_block_id = Some(block.get_id());
                        })
                        .or_insert(SlipSpentStatus::new_on_longest_chain(
                            output.clone(),
                            block.get_id(),
                        ));
                });
            tx.inputs.par_iter().for_each(|input| {
                self.status_map.entry(input.clone()).and_modify(
                    |output_spent_status: &mut SlipSpentStatus| {
                        output_spent_status.longest_chain_spent_block_id = Some(block.get_id());
                    },
                );
            });
        });
    }

    /// Used internally in utxoset to determine the status of a output with respect
    /// to the longest chain. This is useful for validating a output on the longest
    /// chain, and also used when we are trying to determine a output's status in
    /// a fork, in which case we need to know it's status at the common ancestor
    /// block.
    fn longest_chain_spent_status(
        &self,
        output_id: &OutputIdProto,
        block_id: u32,
    ) -> LongestChainSpentTime {
        match &self.status_map.get(output_id) {
            Some(status) => {
                match status.longest_chain_unspent_block_id {
                    Some(longest_chain_unspent_block_id) => {
                        match status.longest_chain_spent_block_id {
                            Some(longest_chain_spent_block_id) => {
                                if longest_chain_unspent_block_id <= block_id
                                    && longest_chain_spent_block_id > block_id
                                {
                                    // There is a spent_block_id but we are interested in the state of the output before it was spent,
                                    // this is useful when looking at the common_ancestor of forks.
                                    LongestChainSpentTime::BetweenUnspentAndSpent
                                } else if longest_chain_unspent_block_id <= block_id {
                                    // The output was already spent
                                    LongestChainSpentTime::AfterSpent
                                } else {
                                    // the output was unspent and spent after this block id
                                    LongestChainSpentTime::NeverExisted
                                }
                            }
                            None => {
                                if longest_chain_unspent_block_id <= block_id {
                                    // The output is created/unspent before this block and we don't have any spent block id
                                    LongestChainSpentTime::BetweenUnspentAndSpent
                                } else {
                                    // The output is created/unspent after this block id
                                    LongestChainSpentTime::BeforeUnspent
                                }
                            }
                        }
                    }
                    // The output is in the utxoset but it's unspent_block_id is set to None, it's been created/unspent but
                    // then set back to None to indicate that the output hasn't been created yet
                    None => LongestChainSpentTime::AfterSpentOrNeverExisted,
                }
            }
            // The output is not in the utxoset, either it was spent and deleted or it never existed
            None => LongestChainSpentTime::AfterSpentOrNeverExisted,
        }
    }

    /// Returns true if the output is Unspent(present in the hashmap and marked Unspent before the
    /// block). The ForkTuple allows us to check for Unspent/Spent status along the fork's
    /// potential new chain more quickly. This can be further optimized in the future.
    pub async fn is_output_spendable_at_block_id(&self, output_id: &OutputIdProto, block_id: u32) -> bool {
        let longest_chain_spent_time = self.longest_chain_spent_status(output_id, block_id);
        longest_chain_spent_time == LongestChainSpentTime::BetweenUnspentAndSpent
    }

    /// Returns true if the output is Unspent(present in the hashmap and marked Unspent before the
    /// root block of a fork). The ForkTuple allows us to check for Unspent/Spent status along the fork's
    /// potential new chain more quickly. This can be further optimized in the future.
    pub async fn is_output_spendable_in_fork_branch(
        &self,
        output_id: &OutputIdProto,
        fork_chains: &ForkChains,
    ) -> bool {
        // first we figure out if the output has been spent at the ancestor block
        let longest_chain_spent_time =
            self.longest_chain_spent_status(output_id, fork_chains.ancestor_block_id);
        if longest_chain_spent_time == LongestChainSpentTime::AfterSpent {
            return false;
        }
        let mut return_val = false;
        if longest_chain_spent_time == LongestChainSpentTime::BetweenUnspentAndSpent {
            // if the output hasn't been spent yet at the ancestor, then we walk the new_chain
            // and check the fork_statuses
            // it must not be spent in this fork
            fork_chains.new_chain.iter().for_each(|block_hash| {
                match &self.status_map.get(output_id) {
                    Some(status) => match status.fork_statuses.get(block_hash) {
                        Some(fork_spend_status) => {
                            if fork_spend_status == &ForkSpentStatus::ForkSpent {
                                return_val = false;
                            }
                        }
                        None => {
                            return_val = true;
                        }
                    },
                    None => {
                        return_val = true;
                    }
                };
            });
        } else {
            // it must be unspent but not spent in this fork
            let mut is_spent = false;
            let mut is_unspent = false;
            fork_chains.new_chain.iter().for_each(|block_hash| {
                match &self.status_map.get(output_id) {
                    Some(status) => match status.fork_statuses.get(block_hash) {
                        Some(fork_spend_status) => {
                            if fork_spend_status == &ForkSpentStatus::ForkSpent {
                                is_spent = true;
                            } else if fork_spend_status == &ForkSpentStatus::ForkUnspent {
                                is_unspent = true;
                            }
                        }
                        None => {}
                    },
                    None => {
                        is_unspent = false;
                    }
                };
            });
            return_val = is_unspent && !is_spent;
        }
        return_val
    }

    /// Loops through all the OutputIdProtos(inputs) and return the amount. This is used to validate
    /// that a transaction is balanced.
    ///
    /// If one of the outputs is not valid, the function returns 0
    pub fn get_total_for_inputs(&self, output_ids: Vec<OutputIdProto>) -> Option<u64> {
        if output_ids.is_empty() {
            None
        } else {
            if let Some(outputs) = output_ids
                .iter()
                .map(|input| self.output_output_from_output_id(&input))
                .collect::<Option<Vec<OutputProto>>>()
            {
                Some(outputs.iter().map(|output| output.amount()).sum())
            } else {
                None
            }
        }
    }

    /// This verifies that the corresponding outputs for the given inputs were all received by
    /// a single address, and, if so, returns that address, otherwise returns None. This is used
    /// to validate that the signer of a transaction is the receiver of all the outputs which
    /// he/she is trying to spend as inputs in a transaction.
    pub fn get_receiver_for_inputs(&self, output_ids: &Vec<OutputIdProto>) -> Option<Vec<u8>> {
        if output_ids.is_empty() {
            None
        } else {
            // output_ids.iter().for_each(|input| {

            // });
            // let _foo = output_ids.iter().map(|input| self.output_output_from_output_id(input)).collect::<Option<Vec<OutputProto>>>();

            // None
            if let Some(outputs) = output_ids
                .iter()
                .map(|input| self.output_output_from_output_id(input).clone())
                //.collect()
                .collect::<Option<Vec<OutputProto>>>()
            {
                Some(outputs[0].address().clone())
                //let first = ;
                // if outputs
                //     .iter()
                //     .all(|output| output.address() == outputs[0].address())
                // {
                //     Some(outputs[0].address().clone())
                // } else {
                //     None
                // }
            } else {
                None
            }
        }
    }
    /// This is used to get the Output(`OutputProto`) which corresponds to a given Input(`OutputIdProto`)
    // pub fn output_output_from_output_id(&self, output_id: &OutputIdProto) -> &OutputProto {
    //     // match self.status_map.get(output_id) {
    //     //     Some(output_output) => Some(&output_output.output_output),
    //     //     None => None,
    //     // }
    //     &self.status_map.get(output_id).unwrap().output_output
    // }

    pub fn output_output_from_output_id(&self, output_id: &OutputIdProto) -> Option<OutputProto> {
        // TODO get rid of the clone here...
        match self.status_map.get(output_id) {
            Some(output_output) => Some(output_output.output_output.clone()),
            None => None,
        }
    }

    /// Computes the fee(leftover of output amount - input amount) for a given transaction.
    pub fn transaction_fees(&self, tx: &TransactionProto) -> u64 {
        let input_amt: u64 = tx
            .inputs
            .iter()
            .map(|input| self.output_output_from_output_id(input).unwrap().amount())
            .sum();

        let output_amt: u64 = tx.outputs.iter().map(|output| output.amount()).sum();

        input_amt - output_amt
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{keypair::Keypair, panda_protos::transaction_proto::TxType, test_utilities::{init_globals_for_tests, mock_block::MockRawBlockForUTXOSet}};

    #[tokio::test]
    async fn roll_forward_and_back_transaction_test() {
        init_globals_for_tests();
        // object under test:
        let mut utxo_set = UtxoSet::new();
        // mock things:
        let keypair = Keypair::new();
        let output_a = OutputProto::new(keypair.get_public_key().clone(), 1);
        let tx_a = TransactionProto::new(vec![], vec![output_a.clone()], TxType::Normal, vec![]);
        let output_a_input = OutputIdProto::new(tx_a.get_hash(), 0);
        let mock_block_a: Box<dyn RawBlock> = Box::new(MockRawBlockForUTXOSet::new(0, [1;32], vec![tx_a]));
        let mock_block_b: Box<dyn RawBlock> = Box::new(MockRawBlockForUTXOSet::new(0, [2;32], vec![]));
        // output should not be spendable yet
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_a_input.clone(), 0).await);
        // rollforward block containing the output
        utxo_set.roll_forward(&mock_block_a);
        // output should be spendable now
        assert!(utxo_set.is_output_spendable_at_block_id(&output_a_input, mock_block_b.get_id()).await);
        // roll back block
        utxo_set.roll_back(&mock_block_a);
        // output should not be spendable again
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_a_input, mock_block_b.get_id()).await);
    }

    #[tokio::test]
    async fn roll_forward_and_back_transaction_on_fork_test() {
        init_globals_for_tests();
        // object under test:
        let mut utxo_set = UtxoSet::new();
        // mock things:
        let keypair = Keypair::new();
        
        // block_a has a single output in it
        let output_a = OutputProto::new(keypair.get_public_key().clone(), 1);
        let tx_a = TransactionProto::new(vec![], vec![output_a], TxType::Normal, vec![]);
        let output_a_input = OutputIdProto::new(tx_a.get_hash(), 0);
        let mock_block_a: Box<dyn RawBlock> = Box::new(MockRawBlockForUTXOSet::new(1, [1; 32], vec![tx_a]));

        // block_b spends the output in block_a and creates a new output
        let output_b = OutputProto::new(keypair.get_public_key().clone(), 1);
        let tx_b = TransactionProto::new(vec![output_a_input.clone()], vec![output_b], TxType::Normal, vec![]);
        let output_b_input = OutputIdProto::new(tx_b.get_hash(), 0);
        let mock_block_b: Box<dyn RawBlock> = Box::new(MockRawBlockForUTXOSet::new(2, [2; 32], vec![tx_b]));
        
        let fork_chains: ForkChains = ForkChains {
            ancestor_block_hash: mock_block_a.get_hash(),
            ancestor_block_id: mock_block_a.get_id(),
            old_chain: vec![],
            new_chain: vec![mock_block_a.get_hash(), mock_block_b.get_hash()],
        };
        // ********* roll_forward tx a  ********* 
        utxo_set.roll_forward(&mock_block_a);

        // a should be spendable, but not b
        // the block prior is BeforeUnspent
        assert_eq!(
            utxo_set.longest_chain_spent_status(&output_a_input, mock_block_a.get_id() - 1),
            LongestChainSpentTime::BeforeUnspent
        );
        // at it's own block id, it has been "unspent"
        assert_eq!(
            utxo_set.longest_chain_spent_status(&output_a_input, mock_block_a.get_id()),
            LongestChainSpentTime::BetweenUnspentAndSpent
        );
        // On the longest chain, for block id above block a, the output should be spendable
        
        assert!(utxo_set.is_output_spendable_at_block_id(&output_a_input, mock_block_b.get_id()).await);
        assert!(utxo_set.is_output_spendable_in_fork_branch(&output_a_input, &fork_chains).await);

        assert!(!utxo_set.is_output_spendable_at_block_id(&output_b_input, mock_block_b.get_id()).await);
        assert!(!utxo_set.is_output_spendable_in_fork_branch(&output_b_input, &fork_chains).await);

        // ********* roll_back tx (as it would if block #2 were rolled back), it should no longer be spendable in the next fork block ********* 
        utxo_set.roll_back(&mock_block_a);
         
        assert_eq!(
            utxo_set.longest_chain_spent_status(&output_a_input, mock_block_a.get_id() - 1),
            LongestChainSpentTime::AfterSpentOrNeverExisted
        );

        assert!(!utxo_set.is_output_spendable_at_block_id(&output_a_input, mock_block_b.get_id()).await);
        assert!(!utxo_set.is_output_spendable_in_fork_branch(&output_a_input, &fork_chains).await);
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_b_input, mock_block_b.get_id()).await);
        assert!(!utxo_set.is_output_spendable_in_fork_branch(&output_b_input, &fork_chains).await);
        
        // ********* roll forward tx like it's in a potential fork ********* 
        utxo_set.roll_forward_on_fork(&mock_block_a);
  
        let fork_chains: ForkChains = ForkChains {
            ancestor_block_hash: mock_block_a.get_hash(),
            ancestor_block_id: mock_block_a.get_id(),
            old_chain: vec![],
            new_chain: vec![mock_block_a.get_hash(), mock_block_b.get_hash()],
        };
        // it should be spendable at block b as a new fork but not spendable at block id 1
        // on the longest chain
        assert!(utxo_set.is_output_spendable_in_fork_branch(&output_a_input, &fork_chains).await);
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_a_input, mock_block_a.get_id()).await);
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_b_input, mock_block_a.get_id()).await);
        assert!(!utxo_set.is_output_spendable_in_fork_branch(&output_b_input, &fork_chains).await);

        // roll forward a tx that spends the output, it's input should become unspendable again and
        // tx b should become spendable on the fork
        utxo_set.roll_forward_on_fork(&mock_block_b);
        
        let fork_chains: ForkChains = ForkChains {
            ancestor_block_hash: mock_block_a.get_hash(),
            ancestor_block_id: mock_block_a.get_id(),
            old_chain: vec![],
            new_chain: vec![mock_block_a.get_hash(), mock_block_b.get_hash(), [3; 32]],
        };
        assert_eq!(
            utxo_set.longest_chain_spent_status(&output_b_input, mock_block_a.get_id() - 1),
            LongestChainSpentTime::AfterSpentOrNeverExisted
        );
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_a_input, mock_block_a.get_id()).await);
        assert!(!utxo_set.is_output_spendable_in_fork_branch(&output_a_input, &fork_chains).await);
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_b_input, mock_block_a.get_id()).await);
        assert!(utxo_set.is_output_spendable_in_fork_branch(&output_b_input, &fork_chains).await);
        // roll back the tx that spent it, it input should become spendable again on the fork and
        // tx b should become unspendable again
        utxo_set.roll_back_on_fork(&mock_block_b);

        let fork_chains: ForkChains = ForkChains {
            ancestor_block_hash: mock_block_a.get_hash(),
            ancestor_block_id: mock_block_a.get_id(),
            old_chain: vec![],
            new_chain: vec![mock_block_a.get_hash(), mock_block_b.get_hash()],
        };
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_a_input, mock_block_a.get_id()).await);
        assert!(utxo_set.is_output_spendable_in_fork_branch(&output_a_input, &fork_chains).await);
        assert!(!utxo_set.is_output_spendable_at_block_id(&output_b_input, mock_block_a.get_id()).await);
        assert!(!utxo_set.is_output_spendable_in_fork_branch(&output_b_input, &fork_chains).await);
    }

//     #[tokio::test]
//     async fn get_total_for_inputs_test() {
//         let keypair = Keypair::new();
//         let (_blockchain, outputs) =
//             test_utilities::mocks::make_mock_blockchain_and_outputs(&keypair, 10).await;
//         let mut inputs = vec![];
//         outputs.iter().for_each(|(output_id, _output_output)| {
//             inputs.push(output_id.clone());
//         });

//         let total = UTXOSET_GLOBAL
//             .clone()
//             .read()
//             .unwrap()
//             .get_total_for_inputs(inputs);
//         assert_eq!(50_000_0000_0000, total.unwrap());
//     }

//     #[tokio::test]
//     async fn get_receiver_for_inputs_test() {
//         let keypair = Keypair::new();
//         let (blockchain, outputs) =
//             test_utilities::mocks::make_mock_blockchain_and_outputs(&keypair, 10).await;
//         outputs.iter().for_each(|(output_id, _output_output)| {
//             let receiver = UTXOSET_GLOBAL
//                 .clone()
//                 .read()
//                 .unwrap()
//                 .get_receiver_for_inputs(&vec![output_id.clone()]);
//             assert_eq!(
//                 &receiver.unwrap(),
//                 &keypair.public_key().serialize().to_vec()
//             );
//         });

//         let (output_id, output_output) = outputs[0].clone();
//         let mock_tx = test_utilities::mocks::make_mock_tx(
//             output_id.clone(),
//             output_output.amount(),
//             keypair.public_key().clone(),
//         );
//         let first_block = blockchain.latest_block().unwrap();
//         let new_block = test_utilities::mocks::make_mock_block_with_tx(
//             first_block.hash(),
//             first_block.id() + 1,
//             mock_tx,
//         );
//         let is_output_spendable = UTXOSET_GLOBAL
//             .clone()
//             .read()
//             .unwrap()
//             .is_output_spendable_at_block_id(&output_id.clone(), new_block.id());
//         assert!(is_output_spendable);
//     }

//     #[tokio::test]
//     async fn output_output_from_output_id_test() {
//         let keypair = Keypair::new();
//         let (_blockchain, outputs) =
//             test_utilities::mocks::make_mock_blockchain_and_outputs(&keypair, 10).await;
//         outputs.iter().for_each(|(output_id, output_output)| {
//             let receiver = UTXOSET_GLOBAL
//                 .clone()
//                 .read()
//                 .unwrap()
//                 .output_output_from_output_id(output_id);
//             assert_eq!(&receiver.unwrap(), output_output);
//         });
//     }

//     #[tokio::test]
//     async fn transaction_fees_test() {
//         let keypair = Keypair::new();
//         let (_blockchain, mut outputs) =
//             test_utilities::mocks::make_mock_blockchain_and_outputs(&keypair, 2).await;

//         let (input, _) = outputs.pop().unwrap();
//         let tx = test_utilities::mocks::make_mock_tx(input, 100, keypair.public_key().clone());

//         let fees = UTXOSET_GLOBAL.clone().read().unwrap().transaction_fees(&tx);
//         assert_eq!(2499999999900, fees);
//     }
}
