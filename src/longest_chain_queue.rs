use crate::types::Sha256Hash;


//include!(concat!(env!("OUT_DIR"), "/constants.rs"));

//const RING_BUFFER_LENGTH: u64 = 2 * EPOCH_LENGTH;

#[derive(Debug, Clone)]
pub struct LongestChainQueue {
    block_hashes: Vec<Sha256Hash>,
}

impl LongestChainQueue {
    /// Create new `LongestChainQueue`
    pub fn new() -> Self {
        LongestChainQueue {
            block_hashes: vec![]
        }
    }
    
    pub fn roll_back(&mut self) -> Sha256Hash {
        self.block_hashes.pop().unwrap()
    }
    
    pub fn roll_forward(&mut self, new_block_hash: &Sha256Hash) {
        self.block_hashes.push(new_block_hash.clone());
    }

    pub fn block_hash_by_id(&self, id: u32) -> &Sha256Hash {
        &self.block_hashes[id as usize]
    }

    pub fn latest_block_id(&self) -> u32 {
        self.block_hashes.len() as u32
    }

    pub fn latest_block_hash(&self) -> &Sha256Hash {
        &self.block_hashes[self.block_hashes.len()]
    }

    pub fn contains_hash_by_block_id(&self, hash: &Sha256Hash, block_id: u32) -> bool {
        self.block_hash_by_id(block_id) == hash
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn longest_chain_queue_test() {
     
    }
}
