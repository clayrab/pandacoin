use crate::{panda_protos::RawBlockProto, types::Sha256Hash};
use std::collections::HashMap;

/// This class is used to store all blocks in a tree structure. We simply use the RawBlock's previous_block_hash 
/// as a pointer to the parent node.
#[derive(Debug)]
pub struct ForkTree {
    fork_tree: HashMap<Sha256Hash, RawBlockProto>,
}

impl ForkTree {
    /// Create new `ForkTree`
    pub fn new() -> Self {
        ForkTree {
            fork_tree: HashMap::new(),
        }
    }

    pub fn insert(&mut self, block_hash: Sha256Hash, block: RawBlockProto) -> Option<&RawBlockProto> {
        self.fork_tree.insert(block_hash, block);
        self.fork_tree.get(&block_hash)
    }

    pub fn remove(&mut self, block_hash: &Sha256Hash) {
        self.fork_tree.remove(block_hash);
    }

    pub fn block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&RawBlockProto> {
        self.fork_tree.get(block_hash)
    }

    pub fn contains_block_hash(&self, block_hash: &Sha256Hash) -> bool {
        self.fork_tree.contains_key(block_hash)
    }
}

#[cfg(test)]
mod test {

    // use super::*;


    // #[test]
    // fn fork_tree_insert_remove_test() {
    //     let mut ft = ForkTree::new();

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