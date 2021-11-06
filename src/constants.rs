// Default block fee for block 1. We simply set this but the actual fees will be 0 because that's just easier
// and a better reflection of reality anyway, although we could in some sense pretend that they are 1 since
// the transactions will be seeding the network anyway.
pub const STARTING_BLOCK_FEE: u64 = 1;
// Number of milliseconds we target to see between blocks.
pub const BLOCK_TIME_TARGET_MS: u64 = 20000;
// Number of blocks used for adjusting the difficulty. we adjust the difficulty for every block.
// We target 15 days which seems long enough that it would be very costly to game but short enough
// that the chain can be reasonable flexible over a short period of time.
pub const NUMBER_OF_BLOCKS_FOR_TARGET_CALC: u64 = 15 * 24 * 60 * (60000 / BLOCK_TIME_TARGET_MS);
