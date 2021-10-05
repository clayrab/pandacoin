use crate::protos::{OutputId, Output};

use crate::protos::TransactionProto;
use crate::protos::transaction_proto::TxType;
impl TransactionProto {


    /// Creates new `Transaction`
    ///
    /// * `broadcast_type` - `TransactionType` of the new `Transaction`
    pub fn new(
        timestamp: u64,
        inputs: Vec<OutputId>,
        outputs: Vec<Output>,
        txtype: TxType,
        message: Vec<u8>,
    ) -> Self {
        TransactionProto {
            timestamp: timestamp,
            inputs: inputs,
            outputs: outputs,
            txtype: txtype as i32,
            message: message,
        }
    }

    /// Returns a timestamp when `Transaction` was created
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns list of `Output` outputs
    pub fn outputs(&self) -> &Vec<Output> {
        &self.outputs
    }

    /// Add a new `Output` to the list of `Slip` outputs
    pub fn add_output(&mut self, slip: Output) {
        self.outputs.push(slip);
    }

    /// Returns list of `OutputId` inputs
    pub fn inputs(&self) -> &Vec<OutputId> {
        &self.inputs
    }

    /// Add a new `OutputId` to the list of `OutputId` inputs
    pub fn add_input(&mut self, slip: OutputId) {
        self.inputs.push(slip);
    }

    /// Returns the message of the `Transaction`
    pub fn message(&self) -> &Vec<u8> {
        &self.message
    }
}