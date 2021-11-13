use crate::types::Sha256Hash;

#[derive(Debug, Clone)]
pub struct LongestChainQueue {
    block_hashes: Vec<Sha256Hash>,
}

impl LongestChainQueue {
    /// Create new `LongestChainQueue`
    pub fn new() -> Self {
        LongestChainQueue {
            block_hashes: vec![],
        }
    }

    pub fn roll_back(&mut self) -> Sha256Hash {
        self.block_hashes.pop().unwrap()
    }

    pub fn roll_forward(&mut self, new_block_hash: &Sha256Hash) {
        self.block_hashes.push(new_block_hash.clone());
    }

    pub fn block_hash_by_id(&self, id: u32) -> Option<&Sha256Hash> {
        println!("block_hash_by_id {}", id);
        if self.block_hashes.len() >= id as usize {
            Some(&self.block_hashes[id as usize - 1])
        } else {
            None
        }
    }

    pub fn latest_block_id(&self) -> u32 {
        self.block_hashes.len() as u32
    }

    pub fn latest_block_hash(&self) -> Option<&Sha256Hash> {
        if self.block_hashes.is_empty() {
            None
        } else {
            Some(&self.block_hashes[self.block_hashes.len()])
        }
    }

    pub fn contains_hash_by_block_id(&self, hash: &Sha256Hash, block_id: u32) -> bool {
        if let Some(block_hash) = self.block_hash_by_id(block_id) {
            block_hash == hash
        } else {
            false
        }
        //self.block_hash_by_id(block_id) == hash
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn longest_chain_queue_test() {}
}
