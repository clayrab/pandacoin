use crate::block::RawBlock;


pub struct BlockFee {}

impl BlockFee {
    pub fn roll_forward(&mut self, block: &RawBlock) {

    }

    pub fn roll_back(&mut self, block: &RawBlock) {
    }
}