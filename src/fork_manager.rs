use std::{collections::{HashMap, HashSet}, sync::Arc};

use sha2::Sha256;

use crate::{block::RawBlock, constants::Constants, types::Sha256Hash, longest_chain_queue::LongestChainQueue, blocks_database::BlocksDatabase, block_fee_manager::{BlockFeeManager, BlockFeeAggregateForkData}};

#[derive(Debug)]
pub struct ForkBlock {
    fork_children: HashSet<Sha256Hash>,
    /// meta data used by burnfee
    block_fee_aggregate_fork_data: BlockFeeAggregateForkData,

}

#[derive(Debug)]
pub struct ForkManagerContext {
    constants: Arc<Constants>,
    //blockchain_ref: Arc<RwLock<Box<dyn AbstractBlockchain + Send + Sync>>>
}

// TODO: it might have been clearer to keep the tips in a separate structure since they arena't really fork_blocks...
// all the logic which uses new() rather than new_with_children and empty fork_children is really to do with this issue...
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
    pub fn new() -> Self {
        ForkBlock {
            fork_children: HashSet::new(),
            block_fee_aggregate_fork_data: BlockFeeAggregateForkData::new(),
        }
    }
    pub fn new_with_children(child_1: Sha256Hash, child_2: Sha256Hash) -> Self {
        // TODO compute block_fee_manager
        let mut fork_children = HashSet::new();
        fork_children.insert(child_1);
        fork_children.insert(child_2);
        ForkBlock {
            fork_children: fork_children,
            block_fee_aggregate_fork_data: BlockFeeAggregateForkData::new(),
        }
    }
}

impl ForkManager {
    pub fn new(genesis_block: &Box<dyn RawBlock>, constants: Arc<Constants>) -> Self {
        // TODO after Rust 1.56 this should work:
        // let fork_blocks: HashMap<Sha256Hash, ForkBlock> = HashMap::from([
        //     (genesis_block.get_hash().clone(),ForkBlock::new()),
        // ]);
        let mut fork_blocks: HashMap<Sha256Hash, ForkBlock> = HashMap::new();
        fork_blocks.insert(genesis_block.get_hash().clone(), ForkBlock::new());
        // the genesis block will point to itself to close the loop at the end.
        let mut fork_block_pointers: HashMap<Sha256Hash, Sha256Hash> = HashMap::new();
        fork_block_pointers.insert(genesis_block.get_hash().clone(), genesis_block.get_hash().clone());

        ForkManager {
            fork_blocks: fork_blocks,
            fork_block_pointers: fork_block_pointers, 
            root: genesis_block.get_hash().clone(),
            context: ForkManagerContext {
                constants
            }
        }
        // we need to create the max_order_root_fork_block, we should just start this at 0 every restart and then
        // roll forward.
    }

    pub async fn get_previous_ancestor_fork_block(&self, block_hash: &Sha256Hash) -> &ForkBlock {
        &self.fork_blocks.get(self.fork_block_pointers.get(block_hash).unwrap()).unwrap()
    }

    pub async fn get_previous_ancestor_fork_block_hash(&self, block_hash: &Sha256Hash) -> Option<&Sha256Hash> {
        println!("get_previous_ancestor_fork_block_hash {:?}", block_hash);
        self.fork_block_pointers.get(block_hash)
    }
    pub fn get_fork_children_of_fork_block(&self, fork_block_hash: &Sha256Hash) -> &HashSet<Sha256Hash> {
        &self.fork_blocks.get(fork_block_hash).unwrap().fork_children
    }

    pub async fn get_next_descendant_fork_block_hash(&self, block_hash: &Sha256Hash, blocks_database: &mut BlocksDatabase) -> Option<&Sha256Hash> {
        println!("get_next_descendant_fork_block_hash {:?}", block_hash);
        let past_fork_block_hash = self.fork_block_pointers.get(block_hash).unwrap();
        println!("past_fork_block_hash {:?}", past_fork_block_hash);
        let mut found_fork_block = None;
        // TODO: these loops are not very rusty...
        // TODO: a better way to do this might be to loop through all the fork_blocks and just find the one which has the same fork_block_pointer as this_block's parent...
        // loop through the fork children and find the correct one(which is an ancestor of this_block)
        for fork_child_hash in &self.fork_blocks.get(past_fork_block_hash).unwrap().fork_children {
            println!("get_next_descendant_fork_block_hash fork_child_hash {:?}", fork_child_hash);
            if fork_child_hash == block_hash {
                return None;
            }
            // start at the parent, we are looking for this_block, we know it is not the fork_child and we will use presence in fork_blocks as a halting condition
            let mut next_parent_block_hash = blocks_database.block_by_hash(&fork_child_hash).unwrap().get_previous_block_hash();
            while !self.fork_blocks.contains_key(&next_parent_block_hash) {
                println!("get_next_descendant_fork_block_hash next parent {:?}", next_parent_block_hash);    
                if &next_parent_block_hash == block_hash {
                    found_fork_block = Some(fork_child_hash);
                    break;
                } else {
                    next_parent_block_hash = blocks_database.block_by_hash(&next_parent_block_hash).unwrap().get_previous_block_hash();
                }
            }
            if found_fork_block.is_some() {
                break;
            }
        }
        found_fork_block
    }

    pub async fn roll_forward_on_fork(&mut self, this_block: &Box<dyn RawBlock>, blocks_database: &mut BlocksDatabase) {
        println!("roll_forward_on_fork {:?}", this_block.get_hash());
        if !self.fork_blocks.contains_key(&this_block.get_previous_block_hash()) {
            // if the parent is not a fork block, it should become one, we need to find it's other child...
            // it's other child is one of it's fork_block's children...
            //let past_fork_block_hash = self.fork_block_pointers.get(&this_block.get_previous_block_hash()).unwrap();
            let found_fork_block_hash = self.get_next_descendant_fork_block_hash(&this_block.get_previous_block_hash(), blocks_database).await;
            // insert the new fork_block with it's two children, this block and the 
            assert!(found_fork_block_hash.is_some());
            let new_inner_fork_block = ForkBlock::new_with_children(this_block.get_hash().clone(), found_fork_block_hash.unwrap().clone());
            self.fork_blocks.insert(this_block.get_previous_block_hash(), new_inner_fork_block);
        
        } else if self.fork_blocks.contains_key(&this_block.get_previous_block_hash()) {
            // else if the parent is a fork block but has no children, remove it 
            let previous_tip = self.fork_blocks.get(&this_block.get_previous_block_hash()).unwrap();
            if previous_tip.fork_children.is_empty() {
                self.fork_blocks.remove(&this_block.get_previous_block_hash());
            }
        }
        // this block becomes a fork block(tip)
        self.fork_blocks.insert(this_block.get_hash().clone(), ForkBlock::new());
    }

    async fn roll_forward_on_fork_priv(&mut self, this_block: &Box<dyn RawBlock>, blocks_database: &mut BlocksDatabase) {
        
        println!("roll_forward_on_fork_priv {:?}", this_block.get_hash());
        if !self.fork_blocks.contains_key(&this_block.get_previous_block_hash()) {
            // if the parent is not a fork block, it should become one, we need to find it's other child...
            // it's other child is one of it's fork_block's children...
            // let past_fork_block_hash = self.fork_block_pointers.get(&this_block.get_previous_block_hash()).unwrap();
            
            // insert the new fork_block with it's two children, this block and the 
            let found_fork_block_hash = self.get_next_descendant_fork_block_hash(&this_block.get_previous_block_hash(), blocks_database).await.unwrap().clone();
            let new_inner_fork_block = ForkBlock::new_with_children(this_block.get_hash().clone(), found_fork_block_hash.clone());
            
            

            let mut next_parent_hash = found_fork_block_hash.clone();
            
            //println!("fork_blocks len {}", self.fork_blocks.len());
            while &next_parent_hash != &this_block.get_previous_block_hash() {
                println!("replace pointer {:?}", next_parent_hash);
                println!("to {:?}", this_block.get_previous_block_hash());
                self.fork_block_pointers
                    .entry(next_parent_hash)
                    .and_modify(|fork_ancestor_hash: &mut Sha256Hash| {
                        *fork_ancestor_hash = this_block.get_previous_block_hash().clone();
                    })
                    .or_insert(this_block.get_previous_block_hash().clone());
                next_parent_hash = blocks_database.block_by_hash(&next_parent_hash).unwrap().get_previous_block_hash();
                
            }
            // insert the new ForkBlock
            self.fork_blocks.insert(this_block.get_previous_block_hash(), new_inner_fork_block);
        
            // point the old ancestor at the new ForkBlock
            let old_ancestor_fork_block_hash = self.fork_block_pointers.get(&this_block.get_previous_block_hash()).unwrap();
            self.fork_blocks.entry(old_ancestor_fork_block_hash.clone()).and_modify(|old_ancestor_fork_block| {
                //println!("")
                old_ancestor_fork_block.fork_children.remove(&found_fork_block_hash);
                old_ancestor_fork_block.fork_children.insert(this_block.get_previous_block_hash().clone());
            });
            // point the new fork at the new ForkBlock
            self.fork_block_pointers.entry(this_block.get_hash().clone()).and_modify(|fork_block_hash| {
                *fork_block_hash = this_block.get_previous_block_hash().clone();
            });
        } else {
            println!("ELSE");
            // insert this block into the ForkBlock's children
            self.fork_blocks.entry(this_block.get_previous_block_hash()).and_modify(|fork_block| {
                fork_block.fork_children.insert(this_block.get_hash().clone());
            });
            // update this block's fork_block_pointer to the one which is already in fork_blocks
            self.fork_block_pointers.entry(this_block.get_hash().clone()).and_modify(|fork_block_hash| {
                *fork_block_hash = this_block.get_previous_block_hash().clone();
            });
        }
    }
    // fn roll_back(&mut self, block: &Box<dyn RawBlock>);
    pub async fn roll_forward(&mut self, block: &Box<dyn RawBlock>, blocks_database: &mut BlocksDatabase, longest_chain_queue: &LongestChainQueue) {
        
        println!("****************** roll forward... {:?}", block.get_hash().clone());
        // this block becomes a fork block(tip)
        self.fork_blocks.insert(block.get_hash().clone(), ForkBlock::new());
        // the fork_block_pointer for this block is the same as it's parent
        self.fork_block_pointers.insert(block.get_hash().clone(), self.fork_block_pointers.get(&block.get_previous_block_hash()).unwrap().clone());

        // unless it is the root(genesis block) the parent should be a fork block with no other children. it should no longer be a fork block,
        // if the parent is a ForkBlock
        if let Some(previous_tip_fork_block) = self.fork_blocks.get(&block.get_previous_block_hash()) {
            if &block.get_previous_block_hash() == &self.root {
                // we don't want to remove previous_tip if it's the root
                println!("building on root block...");
                self.fork_blocks.get_mut(&block.get_previous_block_hash()).unwrap().fork_children.insert(block.get_hash().clone());
            } else if previous_tip_fork_block.fork_children.is_empty() {
                // if previous_tip has no children, remove it.
                println!("remove {:?}", &block.get_previous_block_hash());
                // get the ancestor ForkBlock children and remove the previous child and add the new child 
                self.fork_blocks.get_mut(self.fork_block_pointers.get(block.get_hash()).unwrap()).unwrap().fork_children.remove(&block.get_previous_block_hash());
                self.fork_blocks.get_mut(self.fork_block_pointers.get(block.get_hash()).unwrap()).unwrap().fork_children.insert(block.get_hash().clone());
                // remove the previous_tip_fork_block
                self.fork_blocks.remove(&block.get_previous_block_hash());
                
            } else {
                println!("here...");
                self.roll_forward_on_fork_priv(block, blocks_database).await;
            }
        } else {
            println!("here2...");
            self.roll_forward_on_fork_priv(block, blocks_database).await;
        }

        // if we are an block id > MAX_REORG, move the root forward 
        if block.get_id() >= self.context.constants.get_max_reorg() {
            println!("move root!!");
            // clean all the fork branches from the blockchain
            let reorg_block_id = block.get_id() - (self.context.constants.get_max_reorg() - 1);
            let reorg_block_hash = longest_chain_queue.block_hash_by_id(reorg_block_id).unwrap();
            let reorg_block = blocks_database.block_by_hash(reorg_block_hash).unwrap();
            assert_eq!(reorg_block.get_previous_block_hash(), self.root);
            // find the proper branch to exclude by walking up the fork_blocks until we find the root
            let mut excluded_branch = block.get_hash().clone();
            while self.fork_block_pointers.get(&excluded_branch).unwrap() != &self.root {
                excluded_branch = self.fork_block_pointers.get(&excluded_branch).unwrap().clone();
            }
            println!("excluded_branch {:?}", excluded_branch);
            self.remove_all_children(&self.root.clone(), blocks_database, longest_chain_queue, &excluded_branch);

            self.fork_block_pointers.remove(&self.root);
            self.root = reorg_block_hash.clone();
            if self.fork_blocks.get(reorg_block_hash).is_none() {
                self.fork_blocks.insert(reorg_block_hash.clone(), ForkBlock::new());
            }

            // Update all the fork_block_pointers between the root and main branch
            // this block is not yet in the blocks database so we cannot get it in the while loops, update beforehand and
            // start the loop at the parent.
            self.fork_block_pointers
                .entry(excluded_branch)
                .and_modify(|fork_ancestor_hash: &mut Sha256Hash| {
                    *fork_ancestor_hash = reorg_block_hash.clone();
                })
                .or_insert(reorg_block_hash.clone());
            let mut next_parent_hash = excluded_branch;
            while self.fork_blocks.get(&next_parent_hash).is_none() || next_parent_hash == excluded_branch {
                println!("replace pointer {:?}", next_parent_hash);
                println!("to {:?}", reorg_block_hash);
                self.fork_block_pointers
                    .entry(next_parent_hash)
                    .and_modify(|fork_ancestor_hash: &mut Sha256Hash| {
                        println!("modify...");
                        *fork_ancestor_hash = reorg_block_hash.clone();
                    })
                    .or_insert(reorg_block_hash.clone());
                next_parent_hash = blocks_database.block_by_hash(&next_parent_hash).unwrap().get_previous_block_hash();
                
            }

        }
    }
    
    fn remove_all_children(&mut self, fork_block_hash: &Sha256Hash, blocks_database: &mut BlocksDatabase, longest_chain_queue: &LongestChainQueue, excluded_branch: &Sha256Hash) {
        println!("remove_all_children {:?}", fork_block_hash);
        println!("excluded_branch {:?}", excluded_branch);
        let mut fork_descendant_hashes = self.get_all_descendant_fork_block_hashes(&mut HashSet::new(), fork_block_hash, excluded_branch);
        //fork_descendant_hashes.remove(excluded_branch);
        for fork_hash in fork_descendant_hashes {
            println!("remove_all_children fork_hash {:?}", fork_hash);
            self.fork_blocks.remove(&fork_hash);
            let mut next_parent_hash = blocks_database.block_by_hash(&fork_hash).unwrap().get_previous_block_hash();
            while self.fork_blocks.get(&next_parent_hash).is_none() {
                println!("remove_all_children next_parent_hash {:?}", next_parent_hash);
                next_parent_hash = blocks_database.block_by_hash(&next_parent_hash).unwrap().get_previous_block_hash();
                let _result = blocks_database.remove(&next_parent_hash);
            }
        }
    }
    fn get_all_descendant_fork_block_hashes(&self, fork_blocks: &mut HashSet<Sha256Hash>, fork_block_hash: &Sha256Hash, excluded_branch: &Sha256Hash) -> HashSet<Sha256Hash>{
        println!("get_all_descendant_fork_block_hashes {:?}", fork_block_hash);
        for fork_child_hash in self.fork_blocks.get(fork_block_hash).unwrap().fork_children.iter() {
            if fork_child_hash != excluded_branch {
                fork_blocks.insert(fork_child_hash.clone());
                self.get_all_descendant_fork_block_hashes(fork_blocks, fork_child_hash, excluded_branch);
            }
            
        }

        fork_blocks.to_owned()
    }
}

#[cfg(test)]
mod test {

    use std::sync::Arc;

    use crate::{blocks_database::BlocksDatabase, longest_chain_queue::LongestChainQueue, test_utilities::{mock_block::{MockRawBlockForForkManager, MockRawBlockForBlockchain}, globals_init::make_timestamp_generator_for_test}, block::{RawBlock, PandaBlock}, keypair::Keypair, constants::Constants};

    use super::ForkManager;

    #[tokio::test]
    async fn roll_forward_fork_manager_test() {
        
        let constants = Arc::new(Constants::new_for_test(None, None, None, None, None, Some(5), None));

        let timestamp_generator = make_timestamp_generator_for_test();
        
        let keypair = Keypair::new();
        let genesis_block = PandaBlock::new_genesis_block(
            keypair.get_public_key().clone(),
            timestamp_generator.get_timestamp(),
            1,
        );
        let genesis_block_hash = genesis_block.get_hash().clone();
        let mut longest_chain_queue = LongestChainQueue::new(&genesis_block);
        let mut fork_manager = ForkManager::new(&genesis_block, constants.clone());
        let mut blocks_database =  BlocksDatabase::new(genesis_block);

        // insert block 1
        timestamp_generator.advance(1000);
        let mock_block_1: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            1,
            [1; 32],
            genesis_block_hash,
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_1_hash = mock_block_1.get_hash().clone();
        longest_chain_queue.roll_forward(&mock_block_1.get_hash());
        fork_manager.roll_forward(&mock_block_1, &mut blocks_database, &longest_chain_queue).await;
        blocks_database.insert(mock_block_1.get_hash().clone(), mock_block_1);
        // insert block 2
        timestamp_generator.advance(1000);
        let mock_block_2: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            2,
            [2; 32],
            [1; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_2_hash = mock_block_2.get_hash().clone();
        longest_chain_queue.roll_forward(&mock_block_2.get_hash());
        fork_manager.roll_forward(&mock_block_2, &mut blocks_database, &longest_chain_queue).await;
        blocks_database.insert(mock_block_2.get_hash().clone(), mock_block_2);
        // insert block 3
        timestamp_generator.advance(1000);
        let mock_block_3: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            [3; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_3_hash = mock_block_3.get_hash().clone();
        longest_chain_queue.roll_forward(&mock_block_3.get_hash());
        fork_manager.roll_forward(&mock_block_3, &mut blocks_database, &longest_chain_queue).await;
        blocks_database.insert(mock_block_3.get_hash().clone(), mock_block_3);

        // insert block 4
        timestamp_generator.advance(1000);
        let mock_block_4: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            [4; 32],
            [3; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_4_hash = mock_block_4.get_hash().clone();
        longest_chain_queue.roll_forward(&mock_block_4.get_hash());
        fork_manager.roll_forward(&mock_block_4, &mut blocks_database, &longest_chain_queue).await;
        blocks_database.insert(mock_block_4.get_hash().clone(), mock_block_4);
        
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_4_hash).await;
        let fork_block = fork_manager.get_previous_ancestor_fork_block(&block_4_hash).await;
        assert_eq!(fork_block_hash, Some(&genesis_block_hash));
        assert_eq!(fork_block.fork_children.len(), 1);
        
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_4_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_3_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, Some(&block_4_hash));

        // insert block 3b
        timestamp_generator.advance(1000);
        let mock_block_3b: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            [30; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_3b_hash = mock_block_3b.get_hash().clone();
        longest_chain_queue.roll_forward(&mock_block_3b.get_hash());
        fork_manager.roll_forward(&mock_block_3b, &mut blocks_database, &longest_chain_queue).await;
        blocks_database.insert(mock_block_3b.get_hash().clone(), mock_block_3b);

        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_2_hash).await;
        assert_eq!(fork_block_hash, Some(&genesis_block_hash));
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_3_hash).await;
        assert_eq!(fork_block_hash, Some(&block_2_hash));
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_4_hash).await;
        assert_eq!(fork_block_hash, Some(&block_2_hash));
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_3b_hash).await;
        assert_eq!(fork_block_hash, Some(&block_2_hash));
        let fork_children = fork_manager.get_fork_children_of_fork_block(&genesis_block_hash);
        assert!(fork_children.contains(&block_2_hash));
        let fork_children = fork_manager.get_fork_children_of_fork_block(&genesis_block_hash);
        assert!(!fork_children.contains(&block_4_hash));

        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_4_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_3b_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_3_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, Some(&block_4_hash));
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_2_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);

        // insert block 4b
        timestamp_generator.advance(1000);
        let mock_block_4b: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            4,
            [40; 32],
            [30; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_4b_hash = mock_block_4b.get_hash().clone();
        longest_chain_queue.roll_forward(&mock_block_4b.get_hash());
        fork_manager.roll_forward(&mock_block_4b, &mut blocks_database, &longest_chain_queue).await;
        blocks_database.insert(mock_block_4b.get_hash().clone(), mock_block_4b);

        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_4_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_3b_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, Some(&block_4b_hash));
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_4b_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_4b_hash).await;
        assert_eq!(fork_block_hash, Some(&block_2_hash));

        // insert block 3c
        timestamp_generator.advance(1000);
        let mock_block_3c: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            3,
            [31; 32],
            [2; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_3c_hash = mock_block_3c.get_hash().clone();
        longest_chain_queue.roll_forward(&mock_block_3c.get_hash());
        fork_manager.roll_forward(&mock_block_3c, &mut blocks_database, &longest_chain_queue).await;
        blocks_database.insert(mock_block_3c.get_hash().clone(), mock_block_3c);

        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_4_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_3c_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_3c_hash).await;
        assert_eq!(fork_block_hash, Some(&block_2_hash));
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_3_hash).await.unwrap();
        assert_eq!(fork_block_hash, &block_2_hash);
        let fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_4_hash).await.unwrap();
        assert_eq!(fork_block_hash, &block_2_hash);
        let fork_children = fork_manager.get_fork_children_of_fork_block(&block_2_hash);
        assert!(fork_children.contains(&block_3c_hash));

        //insert block 5
        timestamp_generator.advance(1000);
        let mock_block_5: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            5,
            [5; 32],
            [4; 32],
            timestamp_generator.get_timestamp(),
            vec![],
        ));
        let block_5_hash = mock_block_5.get_hash().clone();
        longest_chain_queue.roll_forward(&mock_block_5.get_hash());
        fork_manager.roll_forward(&mock_block_5, &mut blocks_database, &longest_chain_queue).await;
        blocks_database.insert(mock_block_5.get_hash().clone(), mock_block_5);

        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_5_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, None);
        let fork_block_hash = fork_manager.get_next_descendant_fork_block_hash(&block_4_hash, &mut blocks_database).await;
        assert_eq!(fork_block_hash, Some(&block_5_hash));
        let previous_fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(&block_4_hash).await.unwrap();
        assert_eq!(previous_fork_block_hash, &block_2_hash);
        
    }
}