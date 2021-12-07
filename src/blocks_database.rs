use crate::{block::RawBlock, types::Sha256Hash, fork_manager::ForkManager};
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
        blocks_database.insert(genesis_block.get_hash().clone(), genesis_block);
        BlocksDatabase {
            blocks_database: blocks_database,
        }
    }

    // TODO remove the hash from this function and just get it from the block itself.
    pub fn insert(
        &mut self,
        block_hash: Sha256Hash,
        block: Box<dyn RawBlock>,
    ) -> Option<&Box<dyn RawBlock>> {
        self.blocks_database.insert(block_hash, block);
        self.blocks_database.get(&block_hash)
    }

    pub fn remove(&mut self, block_hash: &Sha256Hash) {
        self.blocks_database.remove(block_hash);
    }

    pub fn block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Box<dyn RawBlock>> {
        self.blocks_database.get(block_hash)
    }

    pub fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.blocks_database.contains_key(block_hash)
    }

    pub fn get_block_hash_by_id_in_fork(&self, block_id: u32, fork_block_hash: &Sha256Hash, fork_manager: &ForkManager) -> &Box<dyn RawBlock> {
        let mut next_ancestor_fork_block_hash = fork_block_hash;
        
        let mut next_ancestor_fork_block_id = self.block_by_hash(next_ancestor_fork_block_hash).unwrap().get_id();
        let mut next_ancestor_block_test_id;
        loop {
            next_ancestor_block_test_id = self.block_by_hash(next_ancestor_fork_block_hash).unwrap().get_id();
            if next_ancestor_block_test_id > block_id {
                break;
            }            
            next_ancestor_fork_block_hash = fork_manager.get_previous_ancestor_fork_block_hash(next_ancestor_fork_block_hash).unwrap();
            next_ancestor_fork_block_id = self.block_by_hash(next_ancestor_fork_block_hash).unwrap().get_id();
        }
        //let next_ancestor_fork_block = blocks_database.block_by_hash(next_ancestor_fork_block_hash).unwrap();
        //let fork_block_id = next_ancestor_fork_block.get_id();
        let mut previous_block = self.block_by_hash(next_ancestor_fork_block_hash).unwrap();
        for _i in 0..(next_ancestor_fork_block_id - block_id) {
            previous_block = self.block_by_hash(&previous_block.get_previous_block_hash()).unwrap();
        }
        &previous_block
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
