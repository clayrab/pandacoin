use std::str::FromStr;

use secp256k1::{PublicKey, Signature};

use crate::{block::RawBlock, types::Sha256Hash};


/// This Mock RawBlock is used for testing Block Fee
pub struct MockRawBlockForBlockFee {
    block_fee: u64,
    timestamp: u64,
}
impl MockRawBlockForBlockFee {
    pub fn new(block_fee: u64, timestamp: u64) -> Self {
        MockRawBlockForBlockFee {
            block_fee,
            timestamp,
        }
    }
}
impl RawBlock for MockRawBlockForBlockFee {
    fn get_block_fee(&self) -> u64 {
        self.block_fee
    }
    fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
}
