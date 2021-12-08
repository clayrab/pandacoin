use std::sync::Arc;

use crate::{block::RawBlock, constants::Constants, fork_manager::ForkManager};

#[derive(Debug)]
pub struct BlockFeeManagerContext {
    constants: Arc<Constants>,
}
///
#[derive(Debug)]
pub struct BlockFeeManager {
    timestamps: Vec<u64>,
    block_fees: Vec<u64>,
    total_fees: u64,
    genesis_block_timestamp: u64,
    context: BlockFeeManagerContext,
}

#[derive(Debug)]
pub struct BlockFeeAggregateForkData {
    timestamps: Vec<u64>,
    block_fees: Vec<u64>,
    total_fees: u64,
}
impl BlockFeeAggregateForkData {
    pub fn new() -> Self {
        BlockFeeAggregateForkData {
            timestamps: vec![],
            block_fees: vec![],
            total_fees: 0,
        }
    }
}

impl BlockFeeAggregateForkData {
    pub fn roll_forward(
        &mut self,
        block_timestamp: u64,
        block_fee: u64,
        number_of_blocks_for_target_calc: u64,
    ) {
        // assert!(block_fee >= self.get_next_fee_old());
        self.timestamps.push(block_timestamp);
        self.block_fees.push(block_fee);
        self.total_fees += block_fee;
        if self.block_fees.len() > number_of_blocks_for_target_calc as usize {
            self.total_fees -= self.block_fees
                [self.block_fees.len() - number_of_blocks_for_target_calc as usize - 1];
        }
    }

    pub fn roll_back(&mut self, number_of_blocks_for_target_calc: u64) {
        self.total_fees -= self.block_fees.last().unwrap();
        self.timestamps.pop();
        self.block_fees.pop();
        if self.block_fees.len() > number_of_blocks_for_target_calc as usize {
            self.total_fees += self.block_fees
                [self.block_fees.len() - number_of_blocks_for_target_calc as usize - 1];
        }
    }
    pub fn get_total_fees(&self) -> u64 {
        self.total_fees
    }
}

impl BlockFeeManager {
    pub fn new(constants: Arc<Constants>, genesis_block_timestamp: u64) -> Self {
        BlockFeeManager {
            timestamps: vec![],
            block_fees: vec![],
            total_fees: 0,
            genesis_block_timestamp,
            context: BlockFeeManagerContext { constants },
        }
    }
    // pub fn roll_forward_on_fork(&mut self, block_timestamp: u64, block_fee: u64, fork_manager: &mut ForkManager) {
    //     // get next descendent ForkBlock
    //     //
    //     assert!(block_fee >= self.get_next_fee_on_fork(fork_manager));

    //     self.timestamps.push(block_timestamp);
    //     self.block_fees.push(block_fee);
    //     self.total_fees += block_fee;
    //     if self.block_fees.len() > self.context.constants.get_number_of_blocks_for_target_calc() as usize {
    //         self.total_fees -= self.block_fees
    //             [self.block_fees.len() - self.context.constants.get_number_of_blocks_for_target_calc() as usize - 1];
    //     }
    // }

    // pub fn roll_back_on_fork(&mut self, _fork_manager: &mut ForkManager) {
    //     self.total_fees -= self.block_fees.last().unwrap();
    //     self.timestamps.pop();
    //     self.block_fees.pop();
    //     if self.block_fees.len() > self.context.constants.get_number_of_blocks_for_target_calc() as usize {
    //         self.total_fees += self.block_fees
    //             [self.block_fees.len() - self.context.constants.get_number_of_blocks_for_target_calc() as usize - 1];
    //     }
    // }

    pub fn roll_forward(&mut self, block_timestamp: u64, block_fee: u64) {
        // TODO we should be able to remove this assert once the blockchain is handling block fee validation, but
        // it's helpful to leave it in for now just so we will get an early warning if something is wrong.
        assert!(block_fee >= self.get_next_fee_old());
        self.timestamps.push(block_timestamp);
        self.block_fees.push(block_fee);
        self.total_fees += block_fee;
        if self.block_fees.len()
            > self
                .context
                .constants
                .get_number_of_blocks_for_target_calc() as usize
        {
            self.total_fees -= self.block_fees[self.block_fees.len()
                - self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc() as usize
                - 1];
        }
    }

    pub fn roll_back(&mut self) {
        self.total_fees -= self.block_fees.last().unwrap();
        self.timestamps.pop();
        self.block_fees.pop();
        if self.block_fees.len()
            > self
                .context
                .constants
                .get_number_of_blocks_for_target_calc() as usize
        {
            self.total_fees += self.block_fees[self.block_fees.len()
                - self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc() as usize
                - 1];
        }
    }

    pub async fn get_next_fee_on_fork(
        &self,
        block: &Box<dyn RawBlock>,
        timestamp: u64,
        _fork_manager: &ForkManager,
    ) -> u64 {
        let (fork_block_hash, fork_block) = _fork_manager
            .get_previous_ancestor_fork_block(block.get_hash())
            .await
            .unwrap();

        let total_fees = self.total_fees;
        if block.get_id() <= 1 {
            self.context.constants.get_starting_block_fee()
        } else if block.get_id()
            > self
                .context
                .constants
                .get_number_of_blocks_for_target_calc() as u32
        {
            // 1) get the timestamp of the block_id - constants.get_number_of_blocks_for_target_calc()
            // 2) diff the two timestamps
            // 3) compute the
            let avg_fee = total_fees
                / self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc();
            // we add block_time_target to the timediff to deal with the fencepost problem, basically pretending that the first block took block_time_target to produce.
            // We do not subtract 1 from the timestamps index because we also want to add one to account for the fence post problem seemlessly from the else case below.
            // i.e. for N blocks, last - first included the time_diff across N-1 blocks, however, self.timestamps.len() is also 1 too large, so these cancel eachother out
            // and we are just left with just "self.timestamps.len() - N" (i.e. the index of the N-1th blocks prior to the last)
            let time_diff = self.context.constants.get_block_time_target_ms()
                + (timestamp
                    - self.timestamps[block.get_id() as usize + 1
                        - self
                            .context
                            .constants
                            .get_number_of_blocks_for_target_calc()
                            as usize]);
            let expected_time_ms = self.context.constants.get_block_time_target_ms()
                * self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc();
            avg_fee * expected_time_ms / time_diff
        } else {
            let avg_fee = total_fees / block.get_id() as u64 + 1;
            // we add block_time_target to the timediff to deal with the fencepost problem, basically pretending that the first block took block_time_target to produce
            let time_diff = self.context.constants.get_block_time_target_ms()
                + (timestamp - self.genesis_block_timestamp);
            let expected_time_ms =
                self.context.constants.get_block_time_target_ms() * (block.get_id() as u64 + 1);
            avg_fee * expected_time_ms / time_diff
        }
    }
    pub fn get_next_fee(&self, block_id: u32, timestamp: u64) -> u64 {
        let total_fees = self.total_fees;
        let next_fee;
        if block_id <= 1 {
            next_fee = self.context.constants.get_starting_block_fee();
        } else if block_id
            > self
                .context
                .constants
                .get_number_of_blocks_for_target_calc() as u32
        {
            let avg_fee = total_fees
                / self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc();
            // we add mock_block_time_target to the timediff to deal with the fencepost problem, basically pretending that the first block took mock_block_time_target to produce.
            // We do not subtract 1 from the timestamps index because we also want to add one to account for the fence post problem seemlessly from the else case below.
            // i.e. for N blocks, last - first included the time_diff across N-1 blocks, however, self.timestamps.len() is also 1 too large, so these cancel eachother out
            // and we are just left with just "self.timestamps.len() - N" (i.e. the index of the N-1th blocks prior to the last)
            let time_diff = self.context.constants.get_block_time_target_ms()
                + (timestamp
                    - self.timestamps[block_id as usize + 1
                        - self
                            .context
                            .constants
                            .get_number_of_blocks_for_target_calc()
                            as usize]);
            let expected_time_ms = self.context.constants.get_block_time_target_ms()
                * self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc();
            next_fee = avg_fee * expected_time_ms / time_diff;
        } else {
            let avg_fee = total_fees / block_id as u64 + 1;
            // we add mock_block_time_target to the timediff to deal with the fencepost problem, basically pretending that the first block took mock_block_time_target to produce
            let time_diff = self.context.constants.get_block_time_target_ms()
                + (timestamp - self.genesis_block_timestamp);
            let expected_time_ms =
                self.context.constants.get_block_time_target_ms() * (block_id as u64 + 1);
            next_fee = avg_fee * expected_time_ms / time_diff;
        }
        next_fee
    }
    pub fn get_next_fee_old(&self) -> u64 {
        let next_fee;
        if self.timestamps.is_empty() {
            next_fee = self.context.constants.get_starting_block_fee();
        } else if self.timestamps.len() == 1 {
            next_fee = self.context.constants.get_starting_block_fee();
        } else if self.timestamps.len()
            > self
                .context
                .constants
                .get_number_of_blocks_for_target_calc() as usize
        {
            let avg_fee = self.total_fees
                / self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc();
            // we add mock_block_time_target to the timediff to deal with the fencepost problem, basically pretending that the first block took mock_block_time_target to produce.
            // We do not subtract 1 from the timestamps index because we also want to add one to account for the fence post problem seemlessly from the else case below.
            // i.e. for N blocks, last - first included the time_diff across N-1 blocks, however, self.timestamps.len() is also 1 too large, so these cancel eachother out
            // and we are just left with just "self.timestamps.len() - N" (i.e. the index of the N-1th blocks prior to the last)
            let time_diff = self.context.constants.get_block_time_target_ms()
                + (self.timestamps.last().unwrap()
                    - self.timestamps[self.timestamps.len()
                        - self
                            .context
                            .constants
                            .get_number_of_blocks_for_target_calc()
                            as usize]);
            let expected_time_ms = self.context.constants.get_block_time_target_ms()
                * self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc();
            next_fee = avg_fee * expected_time_ms / time_diff;
        } else {
            let avg_fee = self.total_fees / self.timestamps.len() as u64;
            // we add mock_block_time_target to the timediff to deal with the fencepost problem, basically pretending that the first block took mock_block_time_target to produce
            let time_diff = self.context.constants.get_block_time_target_ms()
                + (self.timestamps.last().unwrap() - self.timestamps.first().unwrap());
            let expected_time_ms =
                self.context.constants.get_block_time_target_ms() * (self.timestamps.len()) as u64;
            next_fee = avg_fee * expected_time_ms / time_diff;
        }
        next_fee
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_utilities::globals_init::make_timestamp_generator_for_test;

    // #[tokio::test]
    // async fn block_fee_basic_test() {
    //     let mut block_fee_manager = BlockFeeManager::new(Arc::new(Constants::new()));
    //     for i in 1..NUMBER_OF_BLOCKS_FOR_TARGET_CALC * 2 as u64 {
    //         let block_timestamp = i * mock_block_time_target as u64;
    //         block_fee_manager.roll_forward(block_timestamp, 1);
    //         assert_eq!(1, block_fee_manager.get_next_fee());
    //     }
    // }

    #[tokio::test]
    async fn block_fee_big_fees_test() {
        let timestamp_generator = make_timestamp_generator_for_test();

        let mut block_timestamp = timestamp_generator.get_timestamp();
        let mut block_fee_manager =
            BlockFeeManager::new(Arc::new(Constants::new()), block_timestamp);

        let mock_block_time_target = 20000;
        // 0 Add a block at the expected time with the expected fee.
        //let mock_block: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockFee::new(1, block_timestamp));
        block_fee_manager.roll_forward(block_timestamp, 1);
        assert_eq!(block_fee_manager.get_next_fee_old(), 1);

        // 1 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee_old(), 500);

        // 2 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee_old(), 667);

        // 3 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee_old(), 750);

        // 4 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee_old(), 800);

        // 5 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee_old(), 833);

        // 6 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee_old(), 857);

        // 7 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee_old(), 875);

        // 8 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee_old(), 889);

        // Only 8 mock_block_time_target have passed, but the algo assumes mock_block_time_target passed
        // for the first block, so we add another 11 to get to 20 * mock_block_time_target.
        // Then we add another 1999, which should get our total to 10000 total block fee over 20 heartbeats.
        block_timestamp += 11 * mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1999);
        assert_eq!(block_fee_manager.get_next_fee_old(), 500);

        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 889);

        // Replay same as above, but add another 1998(rather than 1999 as above), which should get
        // the total fees to 9999 total  block fee over 20 heartbeats, which should round down the
        // next fee to 499 rather than 500.

        // block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1998, block_timestamp));
        block_fee_manager.roll_forward(block_timestamp, 1998);
        assert_eq!(block_fee_manager.get_next_fee_old(), 499);

        // rollback everything and we should get all the same numbers back.
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 889);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 875);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 857);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 833);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 800);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 750);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 667);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 500);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee_old(), 1);
    }

    #[tokio::test]
    async fn block_fee_big_fees_test_new() {
        let timestamp_generator = make_timestamp_generator_for_test();

        let mut block_timestamp = timestamp_generator.get_timestamp();
        let mut block_fee_manager =
            BlockFeeManager::new(Arc::new(Constants::new()), block_timestamp);

        let mock_block_time_target = 20000;
        // 0 Add a block at the expected time with the expected fee.
        //let mock_block: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockFee::new(1, block_timestamp));
        block_fee_manager.roll_forward(block_timestamp, 1);
        assert_eq!(block_fee_manager.get_next_fee(0, block_timestamp), 1);

        // 1 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee(1, block_timestamp), 500);

        // 2 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee(2, block_timestamp), 667);

        // 3 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee(3, block_timestamp), 750);

        // 4 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee(4, block_timestamp), 800);

        // 5 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee(5, block_timestamp), 833);

        // 6 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee(6, block_timestamp), 857);

        // 7 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee(7, block_timestamp), 875);

        // 8 Add a block at the expected time, but with a huge fee attached.
        block_timestamp += mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1000);
        assert_eq!(block_fee_manager.get_next_fee(8, block_timestamp), 889);

        // Only 8 mock_block_time_target have passed, but the algo assumes mock_block_time_target passed
        // for the first block, so we add another 11 to get to 20 * mock_block_time_target.
        // Then we add another 1999, which should get our total to 10000 total block fee over 20 heartbeats.
        block_timestamp += 11 * mock_block_time_target;
        block_fee_manager.roll_forward(block_timestamp, 1999);
        assert_eq!(block_fee_manager.get_next_fee(9, block_timestamp), 500);

        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(8, block_timestamp), 889);

        // Replay same as above, but add another 1998(rather than 1999 as above), which should get
        // the total fees to 9999 total  block fee over 20 heartbeats, which should round down the
        // next fee to 499 rather than 500.

        // block_fee_manager.roll_forward(&MockRawBlockForBlockFee::new(1998, block_timestamp));
        block_fee_manager.roll_forward(block_timestamp, 1998);
        assert_eq!(block_fee_manager.get_next_fee(9, block_timestamp), 499);

        // rollback everything and we should get all the same numbers back.
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(8, block_timestamp), 889);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(7, block_timestamp), 875);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(6, block_timestamp), 857);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(5, block_timestamp), 833);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(4, block_timestamp), 800);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(3, block_timestamp), 750);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(2, block_timestamp), 667);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(1, block_timestamp), 500);
        block_fee_manager.roll_back();
        assert_eq!(block_fee_manager.get_next_fee(0, block_timestamp), 1);
    }
}
