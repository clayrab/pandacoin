use async_trait::async_trait;
use std::collections::HashMap;

use crate::Error;
use crate::block::RawBlock;
use crate::blockchain::{AbstractBlockchain, AddBlockEvent};
use crate::types::Sha256Hash;

#[derive(Debug)]
pub struct MockBlockchain {
    blocks: HashMap<Sha256Hash, Box<dyn RawBlock>>,
    blocks_vec: Vec<Box<dyn RawBlock>>,
}

impl MockBlockchain {
    pub fn new() -> Self {
        MockBlockchain {
            blocks: HashMap::new(),
            blocks_vec: vec![],
        }
    }
}

#[async_trait]
impl AbstractBlockchain for MockBlockchain {
    fn latest_block(&self) -> Option<&Box<dyn RawBlock>> {
        None
    }
    /// If the block is in the fork
    fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Box<dyn RawBlock>> {
        self.blocks.get(block_hash)
    }
    /// If the block is in the fork
    fn get_block_by_id(&self, block_id: u32) -> Option<&Box<dyn RawBlock>> {
        Some(&self.blocks_vec[block_id as usize])
    }
    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    async fn add_block(&mut self, block: Box<dyn RawBlock>) -> AddBlockEvent {
        let hash: Sha256Hash = block.get_hash().clone();
        //let hash: Sha256Hash = (*block.get_hash().as_ref().unwrap().clone()).try_into().unwrap();
        self.blocks.insert(hash, block);
        AddBlockEvent::Accepted
    }
    async fn remove_block(&mut self, _block_hash: &Sha256Hash) -> Result<(), Error> {
        // remove from fork tree
        Ok(())
    }
}
