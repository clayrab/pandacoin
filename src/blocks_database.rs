use crate::{
    block::RawBlock, fork_manager::ForkManager, longest_chain_queue::LongestChainQueue,
    types::Sha256Hash,
};
use std::collections::HashMap;

/// This class is used to store all blocks in a tree structure. We simply use the RawBlock's previous_block_hash
/// as a pointer to the parent node.
#[derive(Debug)]
pub struct BlocksDatabase {
    blocks_database: HashMap<Sha256Hash, Box<dyn RawBlock>>,
}

impl BlocksDatabase {
    /// Create new `BlocksDatabase`
    // pub fn new(genesis_block: Box<dyn RawBlock>) -> Self {
    //     BlocksDatabase {
    //         blocks_database: HashMap::new(),
    //     }
    // }
    pub fn new(genesis_block: Box<dyn RawBlock>) -> Self {
        let mut blocks_database = HashMap::new();
        blocks_database.insert(*genesis_block.get_hash(), genesis_block);
        BlocksDatabase { blocks_database }
    }

    // TODO remove the hash from this function and just get it from the block itself.
    pub fn insert(&mut self, block: Box<dyn RawBlock>) {
        self.blocks_database.insert(*block.get_hash(), block);
    }

    pub fn remove(&mut self, block_hash: &Sha256Hash) {
        self.blocks_database.remove(block_hash);
    }

    pub fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Box<dyn RawBlock>> {
        self.blocks_database.get(block_hash)
    }

    pub fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.blocks_database.contains_key(block_hash)
    }

    pub fn get_block_hash_from_fork_by_id(
        &self, target_block_id: u32, starting_search_block_hash: &Sha256Hash,
        longest_chain_queue: &LongestChainQueue, fork_manager: &ForkManager,
    ) -> Option<&Box<dyn RawBlock>> {
        let starting_search_block = self.get_block_by_hash(starting_search_block_hash).unwrap();
        let mut fork_block_hash = starting_search_block.get_hash();
        let mut prev_fork_block_hash: &Sha256Hash = starting_search_block.get_hash();
        // loop until we find a block id before the target
        let mut next_block_id = starting_search_block.get_id();
        while next_block_id > target_block_id && fork_block_hash != fork_manager.get_root() {
            prev_fork_block_hash = fork_block_hash;
            fork_block_hash = fork_manager
                .get_previous_ancestor_fork_block_hash(fork_block_hash)
                .unwrap();
            next_block_id = fork_manager
                .get_fork_block(fork_block_hash)
                .unwrap()
                .get_block_id();
        }
        if fork_block_hash == fork_manager.get_root() {
            // we got to the root, get block from the longest chain
            let block_hash = longest_chain_queue
                .get_block_hash_by_id(target_block_id)
                .unwrap();
            Some(self.get_block_by_hash(block_hash).unwrap())
        } else {
            // we are still in a fork, crawl up the parent blocks until we reach the block id we are searching for
            let mut nearest_descendant_block =
                self.get_block_by_hash(prev_fork_block_hash).unwrap();
            while nearest_descendant_block.get_id() > target_block_id {
                nearest_descendant_block = self
                    .get_block_by_hash(&nearest_descendant_block.get_previous_block_hash())
                    .unwrap();
            }
            Some(nearest_descendant_block)
        }
    }
}

#[cfg(test)]
mod test {

    // use super::*;

    // #[test]
    // fn blocks_database_insert_remove_test() {
    //     let mut ft = BlocksDatabase::new();

    //     let block = make_mock_block_empty([0; 32], 0);

    //     if let Some(new_block) = ft.insert(block.hash(), block.clone()) {
    //         assert_eq!(&block, new_block);
    //     }

    //     match ft.block_by_hash(&block.hash()) {
    //         Some(b) => {
    //             assert_eq!(b, &block);
    //         }
    //         None => assert!(false),
    //     }

    //     assert_eq!(ft.contains_block_hash(&block.hash()), true);

    //     ft.remove(&block.hash());

    //     match ft.block_by_hash(&block.hash()) {
    //         Some(_) => assert!(false),
    //         None => assert!(true),
    //     }

    //     assert_eq!(ft.contains_block_hash(&block.hash()), false);
    // }
}
