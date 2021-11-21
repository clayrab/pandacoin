use crate::constants::Constants;

impl Constants {
    pub fn new_for_test(
        starting_block_fee: Option<u64>,
        block_time_target_ms: Option<u64>,
        number_of_blocks_for_target_calc: Option<u64>,
        total_lit: Option<u64>,
        ions_per_lit: Option<u64>,
        max_reorg: Option<u32>,
        payment_delay: Option<u32>,
    ) -> Self {
        Constants {
            STARTING_BLOCK_FEE: starting_block_fee.unwrap_or(1),
            BLOCK_TIME_TARGET_MS: block_time_target_ms.unwrap_or(20000),
            NUMBER_OF_BLOCKS_FOR_TARGET_CALC: number_of_blocks_for_target_calc.unwrap_or(15 * 24 * 60 * (60000 / 20000)),
            TOTAL_LIT: total_lit.unwrap_or(10_000_000_000),
            IONS_PER_LIT: ions_per_lit.unwrap_or(1_000_000_000),
            MAX_REORG: max_reorg.unwrap_or(24 * 60 * (60000 / 20000 as u32)),
            PAYMENT_DELAY: payment_delay.unwrap_or(7 * 24 * 60 * (60000 / 20000 as u32)),
        }
    }
}