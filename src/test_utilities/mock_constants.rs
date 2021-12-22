use crate::constants::Constants;

impl Constants {
    pub fn new_for_test(
        starting_block_fee: Option<u64>, block_time_target_ms: Option<u64>,
        number_of_blocks_for_target_calc: Option<u64>, total_lit: Option<u64>,
        ions_per_lit: Option<u64>, max_reorg: Option<u32>, payment_delay: Option<u32>,
    ) -> Self {
        Constants {
            starting_block_fee: starting_block_fee.unwrap_or(1),
            block_time_target_ms: block_time_target_ms.unwrap_or(20000),
            number_of_blocks_for_target_calc: number_of_blocks_for_target_calc
                .unwrap_or(15 * 24 * 60 * (60000 / 20000)),
            total_lit: total_lit.unwrap_or(10_000_000_000),
            ions_per_lit: ions_per_lit.unwrap_or(1_000_000_000),
            max_reorg: max_reorg.unwrap_or(24 * 60 * (60000 / 20000_u32)),
            payment_delay: payment_delay.unwrap_or(7 * 24 * 60 * (60000 / 20000_u32)),
        }
    }
}
