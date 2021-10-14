use crate::{block::RawBlock, constants::{BLOCK_TIME_TARGET_MS, NUMBER_OF_BLOCKS_FOR_TARGET_CALC, STARTING_BLOCK_FEE}};

/// 
pub struct BlockFeeManager {
    timestamps: Vec<u64>,
    block_fees: Vec<u64>,
    total_fees: u64,
}

impl BlockFeeManager {
    pub fn new() -> Self {
        BlockFeeManager {
            timestamps: vec![],
            block_fees: vec![],
            total_fees: 0,
        }
    }

    pub fn roll_forward(&mut self, block: & dyn RawBlock) {
        self.timestamps.push(block.get_timestamp());
        self.block_fees.push(block.get_block_fee());
        self.total_fees += block.get_block_fee();
        if self.block_fees.len() > NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize {  
            self.total_fees -= self.block_fees[self.block_fees.len() - NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize - 1];
        }
    }

    pub fn roll_back(&mut self) {
        self.timestamps.pop();
        self.block_fees.pop();
        self.total_fees -= self.block_fees.last().unwrap();
        if self.block_fees.len() > NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize {  
            self.total_fees += self.block_fees[self.block_fees.len() - NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize - 1];
        }
    }

    pub fn get_next_fee(&self) -> u64 {
        println!("self.total_fees {}", self.total_fees);
        
        let next_fee;
        if self.timestamps.is_empty() {
            println!("1");
            next_fee = STARTING_BLOCK_FEE;
        } else if self.timestamps.len() == 1 {
            println!("2");
            next_fee = STARTING_BLOCK_FEE;
        } else if self.timestamps.len() > NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize {
            let avg_fee = self.total_fees/NUMBER_OF_BLOCKS_FOR_TARGET_CALC;
            println!("3");
            let time_diff = self.timestamps.last().unwrap() - self.timestamps[self.timestamps.len() - NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize - 1];
            let expected_time_ms = BLOCK_TIME_TARGET_MS * NUMBER_OF_BLOCKS_FOR_TARGET_CALC;
            next_fee = avg_fee * expected_time_ms / time_diff;
        } else {
            let avg_fee = self.total_fees/self.timestamps.len() as u64;
            println!("4");
            let time_diff = self.timestamps.last().unwrap() - self.timestamps.first().unwrap();
            let expected_time_ms = BLOCK_TIME_TARGET_MS * (self.timestamps.len() - 1) as u64;
            println!("{} {} {}", avg_fee, expected_time_ms, time_diff);
            next_fee = avg_fee * expected_time_ms / time_diff;
        }
        next_fee
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // include!(concat!(env!("OUT_DIR"), "/constants.rs"));
    use crate::{constants::BLOCK_TIME_TARGET_MS, test_utilities::mock_block::MockRawBlockForBlockFee};
    use crate::timestamp_generator::TIMESTAMP_GENERATOR_GLOBAL;

    // lazy_static! {
    //     pub static ref TIMESTAMP_GENERATOR_GLOBAL: Box<dyn TimestampGenerator + Send + Sync> = Box::new(MockTimestampGenerator::new());
    // }
    #[tokio::test]
    async fn block_fee_init_test() {
        let mut block_fee_manager = BlockFeeManager::new();

        let block_timestamp = TIMESTAMP_GENERATOR_GLOBAL.get_timestamp();
        let mock_block_1 = MockRawBlockForBlockFee::new(1, block_timestamp);
        block_fee_manager.roll_forward(&mock_block_1);
        assert_eq!(block_fee_manager.get_next_fee(), 1);
        let mock_block_2 = MockRawBlockForBlockFee::new(1, block_timestamp + BLOCK_TIME_TARGET_MS);
        block_fee_manager.roll_forward(&mock_block_2);
        assert_eq!(block_fee_manager.get_next_fee(), 1);
        let mock_block_3 = MockRawBlockForBlockFee::new(1, block_timestamp + BLOCK_TIME_TARGET_MS + BLOCK_TIME_TARGET_MS);
        block_fee_manager.roll_forward(&mock_block_3);
        assert_eq!(block_fee_manager.get_next_fee(), 1);

        let mock_block_4 = MockRawBlockForBlockFee::new(5, block_timestamp + BLOCK_TIME_TARGET_MS + BLOCK_TIME_TARGET_MS);
        block_fee_manager.roll_forward(&mock_block_4);
        assert_eq!(block_fee_manager.get_next_fee(), 2);

    }
    #[tokio::test]
    async fn block_fee_test() {

        let mut block_fee_manager = BlockFeeManager::new();

        let mut mock_blocks = vec![];
        for i in 0..NUMBER_OF_BLOCKS_FOR_TARGET_CALC as u64 {
            let block_timestamp = i * BLOCK_TIME_TARGET_MS as u64/NUMBER_OF_BLOCKS_FOR_TARGET_CALC as u64;
            let mock_block = MockRawBlockForBlockFee::new(i, block_timestamp);
            block_fee_manager.roll_forward(&mock_block);
            mock_blocks.push(mock_block);
        }
        println!("get_next_fee {}", block_fee_manager.get_next_fee());
    }
}
