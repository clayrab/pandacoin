use std::{collections::VecDeque, sync::Arc};

use crate::{
    block::RawBlock, blocks_database::BlocksDatabase, constants::Constants,
    fork_manager::ForkManager, longest_chain_queue::LongestChainQueue,
};

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

#[derive(Clone, Debug)]
pub struct BlockFeeAggregateForkData {
    timestamps: VecDeque<u64>,
    block_fees: VecDeque<u64>,
    total_fees: u64,
}
impl BlockFeeAggregateForkData {
    pub fn new(block: &Box<dyn RawBlock>) -> Self {
        let mut timestamps = VecDeque::new();
        let mut block_fees = VecDeque::new();
        timestamps.push_back(block.get_timestamp());
        block_fees.push_back(block.get_block_fee());
        // TODO use VecDeque::from
        BlockFeeAggregateForkData {
            timestamps,
            block_fees,
            total_fees: block.get_block_fee(),
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
        self.timestamps.push_back(block_timestamp);
        self.block_fees.push_back(block_fee);
        self.total_fees += block_fee;
        if self.block_fees.len() > number_of_blocks_for_target_calc as usize {
            self.total_fees -= self.block_fees
                [self.block_fees.len() - number_of_blocks_for_target_calc as usize - 1];
        }
    }

    pub fn roll_back(&mut self) {
        self.total_fees -= self.block_fees.back().unwrap();
        self.timestamps.pop_back();
        self.block_fees.pop_back();
    }

    pub fn roll_front_forward(&mut self) {
        self.total_fees -= self.block_fees.front().unwrap();
        self.timestamps.pop_front();
        self.block_fees.pop_front();
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

    pub fn roll_forward_old(&mut self, block_timestamp: u64, block_fee: u64) {
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
    pub fn roll_forward(&mut self, root_block: &Box<dyn RawBlock>) {
        self.timestamps.push(root_block.get_timestamp());
        self.block_fees.push(root_block.get_block_fee());
        self.total_fees += root_block.get_block_fee();
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
        next_block: &Box<dyn RawBlock>,
        fork_manager: &ForkManager,
        longest_chain_queue: &LongestChainQueue,
        blocks_database: &BlocksDatabase,
    ) -> u64 {
        if next_block.get_id() <= 1 {
            return self.context.constants.get_starting_block_fee();
        }
        let last_block = blocks_database
            .get_block_by_hash(&next_block.get_previous_block_hash())
            .unwrap();
        //let last_block_id = next_block.get_id() - 1;
        let mut earliest_block_id = 0;
        // we don't subtract 1 here because of fenceposts, we want amount of time passed
        if next_block.get_id()
            > self
                .context
                .constants
                .get_number_of_blocks_for_target_calc() as u32
        {
            earliest_block_id = 1 + next_block.get_id()
                - self
                    .context
                    .constants
                    .get_number_of_blocks_for_target_calc() as u32;
        }
        let mut earliest_block_timestamp = blocks_database
            .get_block_by_hash(
                longest_chain_queue
                    .get_block_hash_by_id(earliest_block_id)
                    .unwrap(),
            )
            .unwrap()
            .get_timestamp();

        let root_block_id = blocks_database
            .get_block_by_hash(fork_manager.get_root())
            .unwrap()
            .get_id();
        if root_block_id < last_block.get_id() {
            earliest_block_timestamp = blocks_database
                .get_block_hash_from_fork_by_id(
                    earliest_block_id,
                    last_block.get_hash(),
                    longest_chain_queue,
                    fork_manager,
                )
                .unwrap()
                .get_timestamp();
        }
        if earliest_block_id == 0 {
            earliest_block_timestamp -= self.context.constants.get_block_time_target_ms();
        }
        let block_count_diff = 1 + last_block.get_id() - earliest_block_id;
        let time_diff = last_block.get_timestamp() - earliest_block_timestamp;

        let mut total_fees = 0;
        let mut previous_fork_block_hash = last_block.get_hash();
        if root_block_id <= last_block.get_id() {
            // loop through all blocks between block and it's previous ForkBlock and get fees
            // let mut previous_fork_block_hash = last_block.get_hash();
            if fork_manager
                .get_fork_block(previous_fork_block_hash)
                .is_none()
            {
                previous_fork_block_hash = fork_manager
                    .get_previous_ancestor_fork_block_hash(&last_block.get_previous_block_hash())
                    .unwrap();
            }

            let mut next_parent_hash = *last_block.get_hash();
            let mut next_parent_block = blocks_database
                .get_block_by_hash(last_block.get_hash())
                .unwrap();

            // loop through blocks before the previous ForkBlock
            // total_fees += next_parent_block.get_block_fee();
            while &next_parent_hash != previous_fork_block_hash {
                total_fees += next_parent_block.get_block_fee();
                next_parent_hash = next_parent_block.get_previous_block_hash();
                next_parent_block = blocks_database
                    .get_block_by_hash(&next_parent_hash)
                    .unwrap();
            }

            // loops through fork_blocks and get total fees.
            let mut previous_fork_block_id;
            let mut fork_block = fork_manager
                .get_fork_block(previous_fork_block_hash)
                .unwrap();
            loop {
                if let Some((_previous_fork_block_hash, previous_fork_block)) = fork_manager
                    .get_previous_ancestor_fork_block(previous_fork_block_hash)
                    .await
                {
                    previous_fork_block_id = previous_fork_block.get_block_id();
                    // if the previous id is too small, we don't want all the aggregate data from fork_block
                    if previous_fork_block_id < earliest_block_id {
                        break;
                    }
                    // Add in from fork_block
                    previous_fork_block_hash = _previous_fork_block_hash;
                    total_fees += fork_block
                        .get_block_fee_aggregate_fork_data()
                        .get_total_fees();
                    fork_block = previous_fork_block;
                } else {
                    total_fees += fork_block
                        .get_block_fee_aggregate_fork_data()
                        .get_total_fees();
                    break;
                }
            }
        }

        // loop from last_previous_fork_block_hash until earliest_block_id and get fees
        if previous_fork_block_hash != fork_manager.get_root() {
            let mut last_previous_fork_block = blocks_database
                .get_block_by_hash(previous_fork_block_hash)
                .unwrap();
            while last_previous_fork_block.get_id() >= earliest_block_id {
                total_fees += last_previous_fork_block.get_block_fee();
                if last_previous_fork_block.get_id() == 0 {
                    break;
                } else {
                    last_previous_fork_block = blocks_database
                        .get_block_by_hash(&last_previous_fork_block.get_previous_block_hash())
                        .unwrap();
                }
            }
        }

        let avg_fee = total_fees / block_count_diff as u64;
        let expected_time_ms =
            self.context.constants.get_block_time_target_ms() * block_count_diff as u64;
        println!("get next fee done. blockid:{} lastid:{} total_fees:{} block_count_diff:{} avg_fee:{} expected_time_ms:{} time_diff:{}", last_block.get_id(), earliest_block_id, total_fees, block_count_diff, avg_fee, expected_time_ms, time_diff);
        avg_fee * expected_time_ms / time_diff
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

    use crate::{
        block::PandaBlock,
        keypair::Keypair,
        longest_chain_queue::LongestChainQueue,
        test_utilities::{
            globals_init::make_timestamp_generator_for_test, mock_block::MockRawBlockForBlockFee,
        },
        types::Sha256Hash,
    };

    async fn add_mock_block_with_fee(
        mock_block_id: u32,
        mock_block_fee: u64,
        mock_block_hash: Sha256Hash,
        mock_parent_hash: Sha256Hash,
        expected_next_fee: u64,
        timestamp: u64,
        fork_manager: &mut ForkManager,
        blocks_database: &mut BlocksDatabase,
        block_fee_manager: &mut BlockFeeManager,
        longest_chain_queue: &mut LongestChainQueue,
    ) {
        // add block 7
        let mock_block: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockFee::new(
            mock_block_id,
            mock_block_fee,
            mock_block_hash,
            mock_parent_hash,
            timestamp,
        ));

        assert_eq!(
            block_fee_manager
                .get_next_fee_on_fork(
                    &mock_block,
                    fork_manager,
                    longest_chain_queue,
                    blocks_database
                )
                .await,
            expected_next_fee
        );
        longest_chain_queue.roll_forward(mock_block.get_hash());
        fork_manager
            .roll_forward(&mock_block, blocks_database, longest_chain_queue)
            .await;
        blocks_database.insert(*mock_block.get_hash(), mock_block);
    }
    #[tokio::test]
    async fn block_fee_big_fees_test_new() {
        let timestamp_generator = make_timestamp_generator_for_test();

        let mut block_fee_manager = BlockFeeManager::new(
            Arc::new(Constants::new()),
            timestamp_generator.get_timestamp(),
        );

        let constants = Arc::new(Constants::new_for_test(
            None,
            Some(20000),
            None,
            None,
            None,
            Some(5),
            None,
        ));

        let keypair = Keypair::new();
        let genesis_block = PandaBlock::new_genesis_block(
            *keypair.get_public_key(),
            timestamp_generator.get_timestamp(),
            1,
        );
        let genesis_block_hash = *genesis_block.get_hash();
        let mut longest_chain_queue = LongestChainQueue::new(&genesis_block);
        let mut fork_manager = ForkManager::new(&genesis_block, constants.clone());
        let mut blocks_database = BlocksDatabase::new(genesis_block);

        timestamp_generator.advance(20000);
        add_mock_block_with_fee(
            1,
            1000,
            [1; 32],
            genesis_block_hash,
            1,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        timestamp_generator.advance(20000);
        add_mock_block_with_fee(
            2,
            1000,
            [2; 32],
            [1; 32],
            500,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        timestamp_generator.advance(20000);
        add_mock_block_with_fee(
            3,
            1000,
            [3; 32],
            [2; 32],
            667,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        timestamp_generator.advance(20000);
        add_mock_block_with_fee(
            4,
            1000,
            [4; 32],
            [3; 32],
            750,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        assert_eq!(fork_manager.get_root(), &genesis_block_hash);
        assert_eq!(
            fork_manager
                .get_fork_block(fork_manager.get_root())
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            1
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[4; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            4000
        );

        timestamp_generator.advance(20000);
        add_mock_block_with_fee(
            5,
            1000,
            [5; 32],
            [4; 32],
            800,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        assert_eq!(fork_manager.get_root(), &[1; 32]);
        assert_eq!(
            fork_manager
                .get_fork_block(fork_manager.get_root())
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            1001
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[5; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            4000
        );

        timestamp_generator.advance(20000);
        add_mock_block_with_fee(
            6,
            1000,
            [6; 32],
            [5; 32],
            833,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        assert_eq!(fork_manager.get_root(), &[2; 32]);
        assert_eq!(
            fork_manager
                .get_fork_block(fork_manager.get_root())
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            2001
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[6; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            4000
        );

        timestamp_generator.advance(20000);
        add_mock_block_with_fee(
            7,
            1000,
            [7; 32],
            [6; 32],
            857,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        timestamp_generator.advance(20000);

        add_mock_block_with_fee(
            8,
            1000,
            [8; 32],
            [7; 32],
            875,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;
        let block_8_time = timestamp_generator.get_timestamp();

        timestamp_generator.advance(20000);

        add_mock_block_with_fee(
            9,
            1000,
            [9; 32],
            [8; 32],
            889,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;
        let block_9_time = timestamp_generator.get_timestamp();

        assert_eq!(fork_manager.get_root(), &[5; 32]);
        assert_eq!(
            fork_manager
                .get_fork_block(fork_manager.get_root())
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            5001
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[9; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            4000
        );

        add_mock_block_with_fee(
            8,
            1000,
            [18; 32],
            [7; 32],
            875,
            block_8_time,
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        assert_eq!(fork_manager.get_root(), &[5; 32]);
        assert_eq!(
            fork_manager
                .get_fork_block(fork_manager.get_root())
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            5001
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[7; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            2000
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[9; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            2000
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[18; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            1000
        );

        add_mock_block_with_fee(
            9,
            1000,
            [19; 32],
            [18; 32],
            889,
            block_9_time,
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        assert_eq!(fork_manager.get_root(), &[5; 32]);
        assert_eq!(
            fork_manager
                .get_fork_block(fork_manager.get_root())
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            5001
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[7; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            2000
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[9; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            2000
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[19; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            2000
        );

        add_mock_block_with_fee(
            10,
            1000,
            [20; 32],
            [19; 32],
            900,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        assert_eq!(fork_manager.get_root(), &[6; 32]);
        assert_eq!(
            fork_manager
                .get_fork_block(fork_manager.get_root())
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            6001
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[7; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            1000
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[9; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            2000
        );
        assert_eq!(
            fork_manager
                .get_fork_block(&[20; 32])
                .unwrap()
                .get_block_fee_aggregate_fork_data()
                .get_total_fees(),
            3000
        );

        add_mock_block_with_fee(
            11,
            1000,
            [21; 32],
            [20; 32],
            999,
            timestamp_generator.get_timestamp(),
            &mut fork_manager,
            &mut blocks_database,
            &mut block_fee_manager,
            &mut longest_chain_queue,
        )
        .await;

        // add_mock_block_with_fee_new(
        //     12,
        //     1000,
        //     [22; 32],
        //     [21; 32],
        //     818,
        //     timestamp_generator.get_timestamp(),
        //     &mut fork_manager,
        //     &mut blocks_database,
        //     &mut block_fee_manager,
        //     &mut longest_chain_queue,
        // ).await;
    }
}
