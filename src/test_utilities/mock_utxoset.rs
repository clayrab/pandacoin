use std::collections::HashMap;

use crate::{
    block::RawBlock,
    blockchain::ForkChains,
    panda_protos::{OutputIdProto, OutputProto, TransactionProto},
    utxoset::AbstractUtxoSet,
};
use async_trait::async_trait;

#[derive(Debug)]
pub struct MockUtxoSet {
    mock_outputs: HashMap<OutputIdProto, OutputProto>,
}

impl MockUtxoSet {
    pub fn new() -> Self {
        MockUtxoSet {
            mock_outputs: HashMap::new(),
        }
    }
    pub fn insert_mock_output(&mut self, output: OutputProto, location: OutputIdProto) {
        self.mock_outputs.insert(location, output);
    }
}

#[async_trait]
impl AbstractUtxoSet for MockUtxoSet {
    fn roll_back_on_fork(&mut self, _block: &Box<dyn RawBlock>) {}
    fn roll_forward_on_fork(&mut self, _block: &Box<dyn RawBlock>) {}
    fn roll_back(&mut self, _block: &Box<dyn RawBlock>) {}
    fn roll_forward(&mut self, _block: &Box<dyn RawBlock>) {}
    async fn is_output_spendable_at_block_id(
        &self,
        _output_id: &OutputIdProto,
        _block_id: u32,
    ) -> bool {
        true
    }
    async fn is_output_spendable_in_fork_branch(
        &self,
        _output_id: &OutputIdProto,
        _fork_chains: &ForkChains,
    ) -> bool {
        true
    }
    fn get_total_for_inputs(&self, _output_ids: Vec<OutputIdProto>) -> Option<u64> {
        Some(0)
    }
    fn get_receiver_for_inputs(&self, _output_ids: &Vec<OutputIdProto>) -> Option<Vec<u8>> {
        Some(vec![0])
    }
    fn output_from_output_id(&self, output_id: &OutputIdProto) -> Option<OutputProto> {
        Some(self.mock_outputs.get(output_id).unwrap().clone())
    }
    fn transaction_fees(&self, _tx: &TransactionProto) -> u64 {
        0
    }
}
