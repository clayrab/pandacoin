use async_trait::async_trait;
use std::collections::HashMap;

use crate::block::RawBlock;
use crate::blockchain::{AddBlockEvent, BlockchainTrait};
use crate::types::Sha256Hash;

#[derive(Debug)]
pub struct MockBlockchain {
    blocks: HashMap<Sha256Hash, Box<dyn RawBlock>>,
}

impl MockBlockchain {
    pub fn new() -> Self {
        MockBlockchain {
            blocks: HashMap::new(),
        }
    }
}

#[async_trait]
impl BlockchainTrait for MockBlockchain {
    fn latest_block(&self) -> Option<&Box<dyn RawBlock>> {
        None
    }
    /// If the block is in the fork
    fn get_block_by_hash(&self, block_hash: &Sha256Hash) -> Option<&Box<dyn RawBlock>> {
        self.blocks.get(block_hash)
    }
    /// Append `Block` to the index of `Blockchain`
    /// These `AddBlockEvent`s will be turned into network responses so peers can figure out
    /// what's going on.
    async fn add_block(&mut self, block: Box<dyn RawBlock>) -> AddBlockEvent {
        let hash: Sha256Hash = block.get_hash();
        //let hash: Sha256Hash = (*block.get_hash().as_ref().unwrap().clone()).try_into().unwrap();
        self.blocks.insert(hash.clone(), block);
        AddBlockEvent::Accepted
    }
}
