use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    block::RawBlock, block_fee_manager::BlockFeeAggregateForkData, blocks_database::BlocksDatabase,
    constants::Constants, longest_chain_queue::LongestChainQueue, types::Sha256Hash,
};

#[derive(Clone, Debug)]
pub struct ForkBlock {
    fork_children: HashSet<Sha256Hash>,
    block_id: u32,
    /// meta data used by burnfee
    block_fee_aggregate_fork_data: BlockFeeAggregateForkData,
}

#[derive(Debug)]
pub struct ForkManagerContext {
    constants: Arc<Constants>,
}

///
/// Fork Manager
///
/// The datastructure maintains a structure like the following:
///
///```text
///     [F]
///      |  [F]
///      |   |  [F]
///      |   | /
///     [F] [F]
///      | /
///     [F]
///      |
///     [R]
///
/// [F] = ForkBlock
/// [R] = Root
/// ```
///
/// The Root will move relative to the Latest Block on the Longest Chain by MAX_REORG blocks.
/// As ForkBlocks becomes the Root, their blocks off of the Longest Chain will be cleaned up
/// from the BlocksDatabase. Blocks may be added at any point in the datastructure and only
/// datastructure will maintain this structure with ForkBlocks only at splits in the chain.
/// We can then leverage this structure to attach aggregated metaaata to ForkBlocks and
/// optimize computation of validity of potential forks.
///
#[derive(Debug)]
pub struct ForkManager {
    /// These are locations where we have a fork in the blockchain, i.e. where a block has
    /// two valid children and we are not yet sure which may become the longest chain
    fork_blocks: HashMap<Sha256Hash, ForkBlock>,
    /// For every block hash, we keep a refernce to the nearest ancestor ForkBlock,
    fork_block_pointers: HashMap<Sha256Hash, Sha256Hash>,
    root: Sha256Hash,
    context: ForkManagerContext,
}

/// A location where we have a fork in the blockchain, i.e. where a block has
/// two valid children and we are not yet sure which may become the longest chain.
/// We will associate aggregated meta-data about the chain between a ForkBlock and it's
/// nearest ancestor ForkBlock at each ForkBlock.
impl ForkBlock {
    pub fn new(block: &Box<dyn RawBlock>) -> Self {
        ForkBlock {
            fork_children: HashSet::new(),
            block_id: block.get_id(),
            block_fee_aggregate_fork_data: BlockFeeAggregateForkData::new(block),
        }
    }
    pub fn get_block_id(&self) -> u32 {
        self.block_id
    }
    pub fn get_block_fee_aggregate_fork_data(&self) -> &BlockFeeAggregateForkData {
        &self.block_fee_aggregate_fork_data
    }

    pub fn roll_forward(&mut self) {
        self.block_id += 1;
    }
    pub fn roll_back(&mut self) {
        self.block_id -= 1;
    }
}

impl ForkManager {
    pub fn new(genesis_block: &Box<dyn RawBlock>, constants: Arc<Constants>) -> Self {
        // TODO after Rust 1.56 this should work:
        // let fork_blocks: HashMap<Sha256Hash, ForkBlock> = HashMap::from([
        //     (genesis_block.get_hash().clone(),ForkBlock::new()),
        // ]);
        let mut fork_blocks: HashMap<Sha256Hash, ForkBlock> = HashMap::new();
        fork_blocks.insert(*genesis_block.get_hash(), ForkBlock::new(genesis_block));
        // the genesis block will point to itself to close the loop at the end.
        let mut fork_block_pointers: HashMap<Sha256Hash, Sha256Hash> = HashMap::new();
        fork_block_pointers.insert(*genesis_block.get_hash(), *genesis_block.get_hash());

        ForkManager {
            fork_blocks,
            fork_block_pointers,
            root: *genesis_block.get_hash(),
            context: ForkManagerContext { constants },
        }
        // we need to create the max_order_root_fork_block, we should just start this at 0 every restart and then
        // roll forward.
    }

    pub async fn get_previous_ancestor_fork_block(
        &self, block_hash: &Sha256Hash,
    ) -> Option<(&Sha256Hash, &ForkBlock)> {
        if let Some(fork_block_pointer_hash) = self.fork_block_pointers.get(block_hash) {
            if fork_block_pointer_hash == block_hash {
                None
            } else {
                self.fork_blocks
                    .get(fork_block_pointer_hash)
                    .map(|fork_block| (fork_block_pointer_hash, fork_block))
            }
        } else {
            None
        }
    }
    pub fn get_root(&self) -> &Sha256Hash {
        &self.root
    }
    pub fn get_fork_block(&self, fork_block_hash: &Sha256Hash) -> Option<&ForkBlock> {
        self.fork_blocks.get(fork_block_hash)
    }

    pub fn get_previous_ancestor_fork_block_hash(
        &self, block_hash: &Sha256Hash,
    ) -> Option<&Sha256Hash> {
        self.fork_block_pointers.get(block_hash)
    }
    pub fn get_fork_children_of_fork_block(
        &self, fork_block_hash: &Sha256Hash,
    ) -> Option<&HashSet<Sha256Hash>> {
        if let Some(fork_block) = self.fork_blocks.get(fork_block_hash) {
            Some(&fork_block.fork_children)
        } else {
            None
        }
    }

    pub async fn get_next_descendant_fork_block_hash(
        &self, block_hash: &Sha256Hash, blocks_database: &BlocksDatabase,
    ) -> Option<&Sha256Hash> {
        if let Some(past_fork_block_hash) = self.fork_block_pointers.get(block_hash) {
            let mut found_fork_block = None;
            // TODO: these loops are not very rusty...
            // TODO: a better way to do this might be to loop through all the fork_blocks and just find the one which has the same fork_block_pointer as this_block's parent...

            // loop through the fork children and find the correct one(which is an ancestor of this_block)
            for fork_child_hash in &self
                .fork_blocks
                .get(past_fork_block_hash)
                .unwrap()
                .fork_children
            {
                if fork_child_hash == block_hash {
                    return None;
                }
                // start at the parent, we are looking for this_block, we know it is not the fork_child and we will use presence in fork_blocks as a halting condition
                let mut next_parent_block_hash = blocks_database
                    .get_block_by_hash(fork_child_hash)
                    .unwrap()
                    .get_previous_block_hash();
                while !self.fork_blocks.contains_key(&next_parent_block_hash) {
                    if &next_parent_block_hash == block_hash {
                        found_fork_block = Some(fork_child_hash);
                        break;
                    } else {
                        next_parent_block_hash = blocks_database
                            .get_block_by_hash(&next_parent_block_hash)
                            .unwrap()
                            .get_previous_block_hash();
                    }
                }
                if found_fork_block.is_some() {
                    break;
                }
            }
            found_fork_block
        } else {
            None
        }
    }

    // recursive function for rolling forward, use recursion with a "tail call" to reverse order of calls while traversing via previous_block_hash
    fn roll_forward_back_of_fork_block(
        from_hash: &Sha256Hash, to_hash: &Sha256Hash, fork_block: &mut ForkBlock,
        blocks_database: &BlocksDatabase,
    ) {
        let next_parent_block = blocks_database.get_block_by_hash(to_hash).unwrap();
        if next_parent_block.get_hash() != from_hash {
            ForkManager::roll_forward_back_of_fork_block(
                from_hash,
                &next_parent_block.get_previous_block_hash(),
                fork_block,
                blocks_database,
            );
            fork_block
                .block_fee_aggregate_fork_data
                .roll_front_forward();
        }
    }

    async fn roll_forward_on_fork(
        &mut self, this_block: &Box<dyn RawBlock>, blocks_database: &mut BlocksDatabase,
    ) {
        if !self
            .fork_blocks
            .contains_key(&this_block.get_previous_block_hash())
        {
            // if the parent is not a fork block, it should become one, we need to find it's other child...
            // it's other child is one of it's fork_block's children...

            // insert the new fork_block with it's two children
            let the_other_branch_fork_block_hash = *self
                .get_next_descendant_fork_block_hash(
                    &this_block.get_previous_block_hash(),
                    blocks_database,
                )
                .await
                .unwrap();
            let mut newly_inserted_descendent_fork_block = self
                .fork_blocks
                .remove(&the_other_branch_fork_block_hash)
                .unwrap();

            let mut new_descendent_fork_block = newly_inserted_descendent_fork_block.clone();

            // loop through all the pointers and roll_back and roll_forward all of the aggregate data structures
            let mut next_parent_hash = the_other_branch_fork_block_hash;

            // loop through all the pointers between the_other_branch_fork_block_hash and the new fork block and point them at
            // the new fork block.
            while &next_parent_hash != &this_block.get_previous_block_hash() {
                // point the fork_block_pointers to the new location
                self.fork_block_pointers
                    .entry(next_parent_hash)
                    .and_modify(|fork_ancestor_hash: &mut Sha256Hash| {
                        *fork_ancestor_hash = this_block.get_previous_block_hash();
                    })
                    .or_insert(this_block.get_previous_block_hash());
                next_parent_hash = blocks_database
                    .get_block_by_hash(&next_parent_hash)
                    .unwrap()
                    .get_previous_block_hash();
                // roll back all the aggregate data...
                newly_inserted_descendent_fork_block.roll_back();
                newly_inserted_descendent_fork_block
                    .block_fee_aggregate_fork_data
                    .roll_back()
            }

            // the descendent_fork_block becomes the newly inserted ForkBlock(with 2 children)
            newly_inserted_descendent_fork_block
                .fork_children
                .insert(*this_block.get_hash());
            newly_inserted_descendent_fork_block
                .fork_children
                .insert(the_other_branch_fork_block_hash);

            // insert the "new" ForkBlock
            self.fork_blocks.insert(
                this_block.get_previous_block_hash(),
                newly_inserted_descendent_fork_block,
            );

            let previous_fork_block_hash = self
                .get_previous_ancestor_fork_block_hash(&this_block.get_previous_block_hash())
                .unwrap();
            ForkManager::roll_forward_back_of_fork_block(
                previous_fork_block_hash,
                &this_block.get_previous_block_hash(),
                &mut new_descendent_fork_block,
                blocks_database,
            );
            self.fork_blocks
                .insert(the_other_branch_fork_block_hash, new_descendent_fork_block);

            // point the old ancestor at the new ForkBlock
            let old_ancestor_fork_block_hash = self
                .fork_block_pointers
                .get(&this_block.get_previous_block_hash())
                .unwrap();
            self.fork_blocks
                .entry(*old_ancestor_fork_block_hash)
                .and_modify(|old_ancestor_fork_block| {
                    old_ancestor_fork_block
                        .fork_children
                        .remove(&the_other_branch_fork_block_hash);
                    old_ancestor_fork_block
                        .fork_children
                        .insert(this_block.get_previous_block_hash());
                });
            // point the new fork at the new ForkBlock
            self.fork_block_pointers
                .entry(*this_block.get_hash())
                .and_modify(|fork_block_hash| {
                    *fork_block_hash = this_block.get_previous_block_hash();
                });
        } else {
            // insert this block into the ForkBlock's children
            self.fork_blocks
                .entry(this_block.get_previous_block_hash())
                .and_modify(|fork_block| {
                    fork_block.fork_children.insert(*this_block.get_hash());
                });
            // update this block's fork_block_pointer to the one which is already in fork_blocks
            self.fork_block_pointers
                .entry(*this_block.get_hash())
                .and_modify(|fork_block_hash| {
                    *fork_block_hash = this_block.get_previous_block_hash();
                });
        }
    }

    /// roll forward
    pub async fn roll_forward(
        &mut self, next_block: &Box<dyn RawBlock>, blocks_database: &mut BlocksDatabase,
        longest_chain_queue: &LongestChainQueue,
    ) {
        println!("** roll forward ** {:?}", next_block.get_hash().clone());
        // the fork_block_pointer for next_block is the same as it's parent's. Insert it into fork_block_pointers.
        self.fork_block_pointers.insert(
            *next_block.get_hash(),
            *self
                .fork_block_pointers
                .get(&next_block.get_previous_block_hash())
                .unwrap(),
        );

        // unless it is the root the parent should be a fork block with no other children. It should no longer be a fork block.
        if let Some(previous_tip_fork_block) =
            self.fork_blocks.get(&next_block.get_previous_block_hash())
        {
            if &next_block.get_previous_block_hash() == &self.root {
                // we don't want to remove previous_tip if it's the root
                self.fork_blocks
                    .get_mut(&next_block.get_previous_block_hash())
                    .unwrap()
                    .fork_children
                    .insert(*next_block.get_hash());
                self.fork_blocks
                    .insert(*next_block.get_hash(), ForkBlock::new(next_block));
            } else if previous_tip_fork_block.fork_children.is_empty() {
                // if previous_tip has no children, remove it.
                // remove the previous_tip_fork_block, roll it forward, and put it back in the new location
                let mut fork_block = self
                    .fork_blocks
                    .remove(&next_block.get_previous_block_hash())
                    .unwrap();
                fork_block.roll_forward();
                fork_block.block_fee_aggregate_fork_data.roll_forward(
                    next_block.get_timestamp(),
                    next_block.get_block_fee(),
                    self.context
                        .constants
                        .get_number_of_blocks_for_target_calc(),
                );
                self.fork_blocks.insert(*next_block.get_hash(), fork_block);

                // get the ancestor ForkBlock children and remove the previous child and add the new child
                self.fork_blocks
                    .get_mut(self.fork_block_pointers.get(next_block.get_hash()).unwrap())
                    .unwrap()
                    .fork_children
                    .remove(&next_block.get_previous_block_hash());
                self.fork_blocks
                    .get_mut(self.fork_block_pointers.get(next_block.get_hash()).unwrap())
                    .unwrap()
                    .fork_children
                    .insert(*next_block.get_hash());
            } else {
                // the previous block is a ForkBlock which has children(and is not the root).
                // TODO Are we excercising this with a test? We need to exercise this in block_fee_manager also.
                println!("TODO Are we excercising this with a test? We need to exercise this in block_fee_manager also.");
                // TODO don't we need to insert the new forkblock as a child of the the one at
                //      next_block's parent?
                self.fork_blocks
                    .insert(*next_block.get_hash(), ForkBlock::new(next_block));
                self.roll_forward_on_fork(next_block, blocks_database).await;
            }
        } else {
            self.fork_blocks
                .insert(*next_block.get_hash(), ForkBlock::new(next_block));
            self.roll_forward_on_fork(next_block, blocks_database).await;
        }

        // if we are at blockid > MAX_REORG, move the root forward
        let root_id = blocks_database
            .get_block_by_hash(&self.root)
            .unwrap()
            .get_id();
        if next_block.get_id() >= self.context.constants.get_max_reorg() + root_id {
            println!("move root!!");
            // TODO Can we get rid of some of these and just get them from reorg_block?
            let reorg_block_id = next_block.get_id() - (self.context.constants.get_max_reorg() - 1);
            let reorg_block_hash = longest_chain_queue
                .get_block_hash_by_id(reorg_block_id)
                .unwrap();
            let reorg_block = blocks_database.get_block_by_hash(reorg_block_hash).unwrap();
            let reorg_block_timestamp = reorg_block.get_timestamp();
            let reorg_block_fee = reorg_block.get_block_fee();

            assert_eq!(reorg_block.get_previous_block_hash(), self.root);
            // find the proper branch to exclude by walking up the fork_blocks until we find the root
            let mut this_branch = *next_block.get_hash();
            while self.fork_block_pointers.get(&this_branch).unwrap() != &self.root {
                this_branch = *self.fork_block_pointers.get(&this_branch).unwrap();
            }
            // clean all the fork branches from the blockchain
            self.remove_all_children(&self.root.clone(), blocks_database, &this_branch);

            // remove the old fork block(will be reinserted later)
            let mut root_fork_block = self.fork_blocks.remove(&self.root).unwrap();
            // remove the forkblock pointer of root
            self.fork_block_pointers.remove(&self.root);

            if self.fork_blocks.get(reorg_block_hash).is_none() {
                // if there isn't already a ForkBlock at the new root, roll the old one forward and replace it's children
                root_fork_block.fork_children = HashSet::new();
                root_fork_block.fork_children.insert(this_branch);
            } else {
                // We need to merge the root ForkBlock and the ForkBlock located at reorg block...
                // We want the aggregate data from the Root ForkBlock, but rolled forward by 1
                // We want to keep the children of the reorg_fork_block
                let reorg_fork_block = self.fork_blocks.remove(reorg_block_hash).unwrap();
                root_fork_block.fork_children = reorg_fork_block.fork_children;
            }
            root_fork_block.roll_forward();
            root_fork_block.block_fee_aggregate_fork_data.roll_forward(
                reorg_block_timestamp,
                reorg_block_fee,
                self.context
                    .constants
                    .get_number_of_blocks_for_target_calc(),
            );
            // remove aggregate data from the fork_children which is now aggregated by the Reorg ForkBlock
            for fork_child_hash in root_fork_block.fork_children.iter() {
                let descendant_fork_block = self.fork_blocks.get_mut(fork_child_hash).unwrap();
                descendant_fork_block
                    .block_fee_aggregate_fork_data
                    .roll_front_forward();
            }
            // re-insert the modified root_fork_block
            self.fork_blocks.insert(*reorg_block_hash, root_fork_block);

            // Replace all the pointers between new root(reorg_block_hash) and this_branch.
            let mut next_parent = next_block;
            if &this_branch != next_block.get_hash() {
                next_parent = blocks_database.get_block_by_hash(&this_branch).unwrap();
            }
            while next_parent.get_hash() != reorg_block_hash {
                self.fork_block_pointers
                    .entry(*next_parent.get_hash())
                    .and_modify(|fork_ancestor_hash: &mut Sha256Hash| {
                        *fork_ancestor_hash = *reorg_block_hash;
                    })
                    .or_insert(*reorg_block_hash);
                next_parent = blocks_database
                    .get_block_by_hash(&next_parent.get_previous_block_hash())
                    .unwrap();
            }

            // replace root and also fix it's pointer(which doesn't get caught by the above while loop)
            self.root = *reorg_block_hash;
            self.fork_block_pointers
                .entry(self.root)
                .and_modify(|fork_ancestor_hash: &mut Sha256Hash| {
                    *fork_ancestor_hash = *reorg_block_hash;
                })
                .or_insert(*reorg_block_hash);
        }
    }

    fn remove_all_children(
        &mut self, fork_block_hash: &Sha256Hash, blocks_database: &mut BlocksDatabase,
        excluded_branch: &Sha256Hash,
    ) {
        let fork_descendant_hashes = self.get_all_descendant_fork_block_hashes(
            &mut HashSet::new(),
            fork_block_hash,
            excluded_branch,
        );
        //fork_descendant_hashes.remove(excluded_branch);
        for fork_hash in fork_descendant_hashes {
            self.fork_blocks.remove(&fork_hash);
            let mut next_parent_hash = blocks_database
                .get_block_by_hash(&fork_hash)
                .unwrap()
                .get_previous_block_hash();
            while self.fork_blocks.get(&next_parent_hash).is_none() {
                next_parent_hash = blocks_database
                    .get_block_by_hash(&next_parent_hash)
                    .unwrap()
                    .get_previous_block_hash();
                let _result = blocks_database.remove(&next_parent_hash);
            }
        }
    }
    fn get_all_descendant_fork_block_hashes(
        &self, fork_blocks: &mut HashSet<Sha256Hash>, fork_block_hash: &Sha256Hash,
        excluded_branch: &Sha256Hash,
    ) -> HashSet<Sha256Hash> {
        for fork_child_hash in self
            .fork_blocks
            .get(fork_block_hash)
            .unwrap()
            .fork_children
            .iter()
        {
            if fork_child_hash != excluded_branch {
                fork_blocks.insert(*fork_child_hash);
                self.get_all_descendant_fork_block_hashes(
                    fork_blocks,
                    fork_child_hash,
                    excluded_branch,
                );
            }
        }

        fork_blocks.to_owned()
    }
}

#[cfg(test)]
mod test {

    use std::sync::Arc;

    use crate::{
        block::{PandaBlock, RawBlock},
        blocks_database::BlocksDatabase,
        constants::Constants,
        keypair::Keypair,
        longest_chain_queue::LongestChainQueue,
        test_utilities::{
            globals_init::make_timestamp_generator_for_test, mock_block::MockRawBlockForBlockchain,
        },
    };

    use super::ForkManager;

    #[tokio::test]
    async fn roll_forward_fork_manager_no_branches_test() {
        let constants = Arc::new(Constants::new_for_test(
            None,
            None,
            None,
            None,
            None,
            Some(5),
            None,
        ));

        let timestamp_generator = make_timestamp_generator_for_test();

        let keypair = Keypair::new();
        let genesis_block = PandaBlock::new_genesis_block(
            *keypair.get_public_key(),
            timestamp_generator.get_timestamp(),
            1,
        );
        let genesis_block_hash = *genesis_block.get_hash();
        let mut longest_chain_queue = LongestChainQueue::new(&genesis_block);
        let mut fork_manager = ForkManager::new(&genesis_block, constants.clone());
        let mut blocks_database = BlocksDatabase::new(genesis_block);

        // insert block 1
        timestamp_generator.advance(1000);
        let mock_block_1: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            1,
            1000,
            [1; 32],
            genesis_block_hash,
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        longest_chain_queue.roll_forward(mock_block_1.get_hash());
        fork_manager
            .roll_forward(&mock_block_1, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_1);

        // insert block 2
        timestamp_generator.advance(1000);
        let mock_block_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            2,
            1000,
            [2; 32],
            [1; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        longest_chain_queue.roll_forward(mock_block_2.get_hash());
        fork_manager
            .roll_forward(&mock_block_2, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_2);

        // insert block 3
        timestamp_generator.advance(1000);
        let mock_block_3: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            1000,
            [3; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_3_hash = *mock_block_3.get_hash();
        longest_chain_queue.roll_forward(mock_block_3.get_hash());
        fork_manager
            .roll_forward(&mock_block_3, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_3);

        // insert block 4
        timestamp_generator.advance(1000);
        let mock_block_4: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            1000,
            [4; 32],
            [3; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_4_hash = *mock_block_4.get_hash();
        longest_chain_queue.roll_forward(mock_block_4.get_hash());
        fork_manager
            .roll_forward(&mock_block_4, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_4);

        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_4_hash);
        let fork_block = fork_manager
            .get_previous_ancestor_fork_block(&block_4_hash)
            .await
            .unwrap();
        assert_eq!(fork_block_hash, Some(&genesis_block_hash));
        assert_eq!(fork_block.1.fork_children.len(), 1);

        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_4_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_3_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, Some(&block_4_hash));

        //insert block 5
        timestamp_generator.advance(1000);
        let mock_block_5: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            5,
            1000,
            [5; 32],
            [4; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        longest_chain_queue.roll_forward(mock_block_5.get_hash());
        fork_manager
            .roll_forward(&mock_block_5, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_5);
    }

    #[tokio::test]
    async fn roll_forward_fork_manager_test() {
        let constants = Arc::new(Constants::new_for_test(
            None,
            None,
            None,
            None,
            None,
            Some(5),
            None,
        ));

        let timestamp_generator = make_timestamp_generator_for_test();

        let keypair = Keypair::new();
        let genesis_block = PandaBlock::new_genesis_block(
            *keypair.get_public_key(),
            timestamp_generator.get_timestamp(),
            1,
        );
        let genesis_block_hash = *genesis_block.get_hash();
        let mut longest_chain_queue = LongestChainQueue::new(&genesis_block);
        let mut fork_manager = ForkManager::new(&genesis_block, constants.clone());
        let mut blocks_database = BlocksDatabase::new(genesis_block);

        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&genesis_block_hash)
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&genesis_block_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&genesis_block_hash),
            Some(&genesis_block_hash)
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&genesis_block_hash)
                .unwrap()
                .get_block_id(),
            0
        );

        // insert block 1
        timestamp_generator.advance(1000);
        let mock_block_1: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            1,
            1000,
            [1; 32],
            genesis_block_hash,
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_1_hash = *mock_block_1.get_hash();
        longest_chain_queue.roll_forward(mock_block_1.get_hash());
        fork_manager
            .roll_forward(&mock_block_1, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_1);

        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&genesis_block_hash)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&block_1_hash)
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&genesis_block_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_1_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_1_hash),
            Some(&genesis_block_hash)
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&genesis_block_hash)
                .unwrap()
                .get_block_id(),
            0
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&block_1_hash)
                .unwrap()
                .get_block_id(),
            1
        );

        // insert block 2
        timestamp_generator.advance(1000);
        let mock_block_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            2,
            1000,
            [2; 32],
            [1; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_2_hash = *mock_block_2.get_hash();
        longest_chain_queue.roll_forward(mock_block_2.get_hash());
        fork_manager
            .roll_forward(&mock_block_2, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_2);

        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&genesis_block_hash)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            fork_manager.get_fork_children_of_fork_block(&block_1_hash),
            None
        );
        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&block_2_hash)
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&genesis_block_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_1_hash, &blocks_database)
                .await,
            Some(&block_2_hash)
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_2_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_1_hash),
            Some(&genesis_block_hash)
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&genesis_block_hash)
                .unwrap()
                .get_block_id(),
            0
        );
        assert!(fork_manager.get_fork_block(&block_1_hash).is_none());
        assert_eq!(
            fork_manager
                .get_fork_block(&block_2_hash)
                .unwrap()
                .get_block_id(),
            2
        );

        // insert block 3
        timestamp_generator.advance(1000);
        let mock_block_3: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            1000,
            [3; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_3_hash = *mock_block_3.get_hash();
        longest_chain_queue.roll_forward(mock_block_3.get_hash());
        fork_manager
            .roll_forward(&mock_block_3, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_3);

        // insert block 4
        timestamp_generator.advance(1000);
        let mock_block_4: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            1000,
            [4; 32],
            [3; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_4_hash = *mock_block_4.get_hash();
        longest_chain_queue.roll_forward(mock_block_4.get_hash());
        fork_manager
            .roll_forward(&mock_block_4, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_4);

        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_4_hash);
        let fork_block = fork_manager
            .get_previous_ancestor_fork_block(&block_4_hash)
            .await
            .unwrap();
        assert_eq!(fork_block_hash, Some(&genesis_block_hash));
        assert_eq!(fork_block.1.fork_children.len(), 1);

        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_4_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_3_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, Some(&block_4_hash));

        // insert block 3b
        timestamp_generator.advance(1000);
        let mock_block_3b: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            1000,
            [30; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_3b_hash = *mock_block_3b.get_hash();
        longest_chain_queue.roll_forward(mock_block_3b.get_hash());
        fork_manager
            .roll_forward(&mock_block_3b, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_3b);

        let fork_block_hash = fork_manager
            .get_previous_ancestor_fork_block_hash(&block_2_hash)
            .unwrap();
        assert_eq!(fork_block_hash, &genesis_block_hash);
        let fork_block_hash = fork_manager
            .get_previous_ancestor_fork_block_hash(&block_3_hash)
            .unwrap();
        assert_eq!(fork_block_hash, &block_2_hash);
        let fork_block_hash = fork_manager
            .get_previous_ancestor_fork_block_hash(&block_4_hash)
            .unwrap();
        assert_eq!(fork_block_hash, &block_2_hash);
        let fork_block_hash = fork_manager
            .get_previous_ancestor_fork_block_hash(&block_3b_hash)
            .unwrap();
        assert_eq!(fork_block_hash, &block_2_hash);
        let fork_children = fork_manager
            .get_fork_children_of_fork_block(&genesis_block_hash)
            .unwrap();
        assert!(fork_children.contains(&block_2_hash));
        let fork_children = fork_manager
            .get_fork_children_of_fork_block(&genesis_block_hash)
            .unwrap();
        assert!(!fork_children.contains(&block_4_hash));

        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_4_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_3b_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_3_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, Some(&block_4_hash));
        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_2_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);

        // insert block 4b
        timestamp_generator.advance(1000);
        let mock_block_4b: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            1000,
            [40; 32],
            [30; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_4b_hash = *mock_block_4b.get_hash();
        longest_chain_queue.roll_forward(mock_block_4b.get_hash());
        fork_manager
            .roll_forward(&mock_block_4b, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_4b);

        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_4_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_3b_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, Some(&block_4b_hash));
        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_4b_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_4b_hash);
        assert_eq!(fork_block_hash, Some(&block_2_hash));

        // insert block 3c
        timestamp_generator.advance(1000);
        let mock_block_3c: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            1000,
            [31; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_3c_hash = *mock_block_3c.get_hash();
        longest_chain_queue.roll_forward(mock_block_3c.get_hash());
        fork_manager
            .roll_forward(&mock_block_3c, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_3c);

        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_4_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager
            .get_next_descendant_fork_block_hash(&block_3c_hash, &blocks_database)
            .await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_3c_hash);
        assert_eq!(fork_block_hash, Some(&block_2_hash));
        let fork_block_hash = fork_manager
            .get_previous_ancestor_fork_block_hash(&block_3_hash)
            .unwrap();
        assert_eq!(fork_block_hash, &block_2_hash);
        let fork_block_hash = fork_manager
            .get_previous_ancestor_fork_block_hash(&block_4_hash)
            .unwrap();
        assert_eq!(fork_block_hash, &block_2_hash);
        let fork_children = fork_manager
            .get_fork_children_of_fork_block(&block_2_hash)
            .unwrap();
        assert!(fork_children.contains(&block_3c_hash));

        //insert block 5. This will move the root.
        timestamp_generator.advance(1000);
        let mock_block_5: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            5,
            1000,
            [5; 32],
            [4; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_5_hash = *mock_block_5.get_hash();
        longest_chain_queue.roll_forward(mock_block_5.get_hash());
        fork_manager
            .roll_forward(&mock_block_5, &mut blocks_database, &longest_chain_queue)
            .await;
        blocks_database.insert(mock_block_5);

        assert_eq!(
            fork_manager.get_fork_children_of_fork_block(&genesis_block_hash),
            None
        );
        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&block_1_hash)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&block_2_hash)
                .unwrap()
                .len(),
            3
        );
        for fork_child_hash in fork_manager
            .get_fork_children_of_fork_block(&block_2_hash)
            .unwrap()
            .iter()
        {
            assert!(
                fork_child_hash == &block_5_hash
                    || fork_child_hash == &block_4b_hash
                    || fork_child_hash == &block_3c_hash
            );
        }
        assert_eq!(
            fork_manager.get_fork_children_of_fork_block(&block_3_hash),
            None
        );
        assert_eq!(
            fork_manager.get_fork_children_of_fork_block(&block_4_hash),
            None
        );
        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&block_5_hash)
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            fork_manager.get_fork_children_of_fork_block(&block_3b_hash),
            None
        );
        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&block_3c_hash)
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            fork_manager
                .get_fork_children_of_fork_block(&block_4b_hash)
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&genesis_block_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_1_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_2_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_3_hash, &blocks_database)
                .await,
            Some(&block_5_hash)
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_4_hash, &blocks_database)
                .await,
            Some(&block_5_hash)
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_5_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_3b_hash, &blocks_database)
                .await,
            Some(&block_4b_hash)
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_3c_hash, &blocks_database)
                .await,
            None
        );
        assert_eq!(
            fork_manager
                .get_next_descendant_fork_block_hash(&block_4b_hash, &blocks_database)
                .await,
            None
        );

        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&genesis_block_hash),
            None
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_1_hash),
            Some(&block_1_hash)
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_2_hash),
            Some(&block_1_hash)
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_3_hash),
            Some(&block_2_hash)
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_4_hash),
            Some(&block_2_hash)
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_5_hash),
            Some(&block_2_hash)
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_3b_hash),
            Some(&block_2_hash)
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_3c_hash),
            Some(&block_2_hash)
        );
        assert_eq!(
            fork_manager.get_previous_ancestor_fork_block_hash(&block_4b_hash),
            Some(&block_2_hash)
        );

        assert!(fork_manager.get_fork_block(&genesis_block_hash).is_none());
        assert_eq!(
            fork_manager
                .get_fork_block(&block_1_hash)
                .unwrap()
                .fork_children
                .len(),
            1
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&block_2_hash)
                .unwrap()
                .fork_children
                .len(),
            3
        );
        assert!(fork_manager.get_fork_block(&block_3_hash).is_none());
        assert!(fork_manager.get_fork_block(&block_4_hash).is_none());
        assert_eq!(
            fork_manager
                .get_fork_block(&block_5_hash)
                .unwrap()
                .fork_children
                .len(),
            0
        );
        assert!(fork_manager.get_fork_block(&block_3b_hash).is_none());
        assert_eq!(
            fork_manager
                .get_fork_block(&block_3c_hash)
                .unwrap()
                .fork_children
                .len(),
            0
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&block_4b_hash)
                .unwrap()
                .fork_children
                .len(),
            0
        );

        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    1,
                    &block_1_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_1_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    1,
                    &block_2_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_1_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    2,
                    &block_2_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_2_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    1,
                    &block_3_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_1_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    2,
                    &block_3_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_2_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    3,
                    &block_3_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_3_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    1,
                    &block_4_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_1_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    2,
                    &block_4_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_2_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    3,
                    &block_4_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_3_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    4,
                    &block_4_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_4_hash
        );

        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    1,
                    &block_4b_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_1_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    2,
                    &block_4b_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_2_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    3,
                    &block_4b_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_3b_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    4,
                    &block_4b_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_4b_hash
        );
        assert_eq!(
            blocks_database
                .get_block_hash_from_fork_by_id(
                    3,
                    &block_5_hash,
                    &longest_chain_queue,
                    &fork_manager
                )
                .unwrap()
                .get_hash(),
            &block_3_hash
        );

        // TODO: This assert causes an infinite loop because block 0 is below the root, it woudl be nice if it didn't do that:
        //       assert_eq!(blocks_database.get_block_hash_from_fork_by_id(0, &block_5_hash, &fork_manager).unwrap().get_hash(), &block_3_hash);
    }
}
