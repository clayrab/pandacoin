use crate::{
    block::RawBlock,
    constants::{BLOCK_TIME_TARGET_MS, NUMBER_OF_BLOCKS_FOR_TARGET_CALC, STARTING_BLOCK_FEE},
};

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

    pub fn roll_forward(&mut self, block: &dyn RawBlock) {
        // TODO we should be able to remove this assert once the blockchain is handling block fee validation, but
        // it's helpful to leave it in for now just so we will get an early warning if something is wrong.
        assert!(block.get_block_fee() >= self.get_next_fee());
        self.timestamps.push(block.get_timestamp());
        self.block_fees.push(block.get_block_fee());
        self.total_fees += block.get_block_fee();
        if self.block_fees.len() > NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize {
            self.total_fees -= self.block_fees
                [self.block_fees.len() - NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize - 1];
        }
    }

    pub fn roll_back(&mut self) {
        self.total_fees -= self.block_fees.last().unwrap();
        self.timestamps.pop();
        self.block_fees.pop();
        if self.block_fees.len() > NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize {
            self.total_fees += self.block_fees
                [self.block_fees.len() - NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize - 1];
        }
    }

    pub fn get_next_fee(&self) -> u64 {
        let next_fee;
        if self.timestamps.is_empty() {
            next_fee = STARTING_BLOCK_FEE;
        } else if self.timestamps.len() == 1 {
            next_fee = STARTING_BLOCK_FEE;
        } else if self.timestamps.len() > NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize {
            let avg_fee = self.total_fees / NUMBER_OF_BLOCKS_FOR_TARGET_CALC;
            // we add BLOCK_TIME_TARGET_MS to the timediff to deal with the fencepost problem, basically pretending that the first block took BLOCK_TIME_TARGET_MS to produce.
            // We do not subtract 1 from the timestamps index because we also want to add one to account for the fence post problem seemlessly from the else case below.
            // i.e. for N blocks, last - first included the time_diff across N-1 blocks, however, self.timestamps.len() is also 1 too large, so these cancel eachother out
            // and we are just left with just "self.timestamps.len() - N" (i.e. the index of the N-1th blocks prior to the last)
            let time_diff = BLOCK_TIME_TARGET_MS
                + (self.timestamps.last().unwrap()
                    - self.timestamps
                        [self.timestamps.len() - NUMBER_OF_BLOCKS_FOR_TARGET_CALC as usize]);
            let expected_time_ms = BLOCK_TIME_TARGET_MS * NUMBER_OF_BLOCKS_FOR_TARGET_CALC;
            next_fee = avg_fee * expected_time_ms / time_diff;
        } else {
            let avg_fee = self.total_fees / self.timestamps.len() as u64;
            // we add BLOCK_TIME_TARGET_MS to the timediff to deal with the fencepost problem, basically pretending that the first block took BLOCK_TIME_TARGET_MS to produce
            let time_diff = BLOCK_TIME_TARGET_MS
                + (self.timestamps.last().unwrap() - self.timestamps.first().unwrap());
            let expected_time_ms = BLOCK_TIME_TARGET_MS * (self.timestamps.len()) as u64;
            next_fee = avg_fee * expected_time_ms / time_diff;
        }
        next_fee
    }
}

#[cfg(test)]
mod test {
    use super::*;
    
    use crate::test_utilities::globals_init::make_timestamp_generator_for_test;
    use crate::test_utilities::mock_block::MockRawBlockForBlockFee;
    use crate::{
        constants::BLOCK_TIME_TARGET_MS,
    };

    #[tokio::test]
    async fn block_fee_basic_test() {
        let mut block_fee_manager = BlockFeeManager::new();
        for i in 1..NUMBER_OF_BLOCKS_FOR_TARGET_CALC * 2 as u64 {
            let block_timestamp = i * BLOCK_TIME_TARGET_MS as u64;
            block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1, block_timestamp));
            assert_eq!(1, block_fee_manager.get_next_fee());
        }
    }

    #[tokio::test]
    async fn block_fee_big_fees_test() {
        let timestamp_generator = make_timestamp_generator_for_test();
        let mut block_fee_manager = BlockFeeManager::new();
        let mut block_timestamp = timestamp_generator.get_timestamp();

        // 0 Add a block at the expected time with the expected fee.
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 1);

        // 1 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1000, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 500);

        // 2 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1000, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 667);

        // 3 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1000, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 750);

        // 4 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1000, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 800);

        // 5 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1000, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 833);

        // 6 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1000, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 857);

        // 7 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1000, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 875);

        // 8 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1000, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 889);

        // Only 8 BLOCK_TIME_TARGET_MS have passed, but the algo assumes BLOCK_TIME_TARGET_MS passed
        // for the first block, so we add another 11 to get to 20 * BLOCK_TIME_TARGET_MS.
        // Then we add another 1999, which should get our total to 10000 total block fee over 20 heartbeats.
        block_timestamp += 11 * BLOCK_TIME_TARGET_MS;
        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1999, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 500);

        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 889);

        // Replay same as above, but add another 1998(rather than 1999 as above), which should get
        // the total fees to 9999 total  block fee over 20 heartbeats, which should round down the
        // next fee to 499 rather than 500.

        block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1998, block_timestamp));
        assert_eq!(block_fee_manager.get_next_fee(), 499);

        // rollback everything and we should get all the same numbers back.
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 889);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 875);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 857);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 833);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 800);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 750);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 667);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 500);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(), 1);
    }
}
