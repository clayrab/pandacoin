// Default block fee for block 1. We simply set this but the actual fees will be 0 because that's just easier
// and a better reflection of reality anyway, although we could in some sense pretend that they are 1 since
// the transactions will be seeding the network anyway.
const STARTING_BLOCK_FEE: u64 = 1;

// Number of milliseconds we target to see between blocks.
const BLOCK_TIME_TARGET_MS: u64 = 20000;

// Number of blocks used for adjusting the difficulty. we adjust the difficulty for every block.
// We target 15 days which seems long enough that it would be very costly to game but short enough
// that the chain can be reasonable flexible over a short period of time.
const NUMBER_OF_BLOCKS_FOR_TARGET_CALC: u64 = 15 * 24 * 60 * (60000 / BLOCK_TIME_TARGET_MS);

// We want 10B total to match the population of earth and we allow as much precision
// as will fit nicely into a uint64
const TOTAL_LIT: u64 = 10_000_000_000;
const IONS_PER_LIT: u64 = 1_000_000_000;

// Amount of blocks beyond which point we will no longer consider forks.
// **** For the sake of performance, the PAYMENT_DELAY  period must be greater than or equal to this length! ****
const MAX_REORG: u32 = 24 * 60 * (60000 / BLOCK_TIME_TARGET_MS as u32); // one day

// This is the amount of time we wait before making payments to Block Producers or Automatic Staking.
// Dealing with forking for Automatic Staking would add considerable complexity to validation of forks, therefore
// we simply accept that this will be greater than the maximum reorganization length and can gain considerable
// compute/memory performance and a simpler implementation.
// **** PAYMENT_DELAY must be greater than or equal to MAX_REORG!!!! ****
const PAYMENT_DELAY: u32 = 7 * 24 * 60 * (60000 / BLOCK_TIME_TARGET_MS as u32); // one week

#[derive(Debug)]
pub struct Constants {
    pub starting_block_fee: u64,
    pub block_time_target_ms: u64,
    pub number_of_blocks_for_target_calc: u64,
    pub total_lit: u64,
    pub ions_per_lit: u64,
    pub max_reorg: u32,
    pub payment_delay: u32,
}

impl Constants {
    pub fn new() -> Self {
        Constants {
            starting_block_fee: STARTING_BLOCK_FEE,
            block_time_target_ms: BLOCK_TIME_TARGET_MS,
            number_of_blocks_for_target_calc: NUMBER_OF_BLOCKS_FOR_TARGET_CALC,
            total_lit: TOTAL_LIT,
            ions_per_lit: IONS_PER_LIT,
            max_reorg: MAX_REORG,
            payment_delay: PAYMENT_DELAY,
        }
    }
    pub fn get_starting_block_fee(&self) -> u64 {
        self.starting_block_fee
    }
    pub fn get_block_time_target_ms(&self) -> u64 {
        self.block_time_target_ms
    }
    pub fn get_number_of_blocks_for_target_calc(&self) -> u64 {
        self.number_of_blocks_for_target_calc
    }
    pub fn get_total_lit(&self) -> u64 {
        self.total_lit
    }
    pub fn get_ions_per_lit(&self) -> u64 {
        self.ions_per_lit
    }
    pub fn get_max_reorg(&self) -> u32 {
        self.max_reorg
    }
    pub fn get_payment_delay(&self) -> u32 {
        self.payment_delay
    }
}
