use crate::panda_protos::{OutputIdProto, OutputProto};

use crate::panda_protos::TransactionProto;
use crate::panda_protos::transaction_proto::TxType;
impl TransactionProto {


    /// Creates new `Transaction`
    ///
    /// * `broadcast_type` - `TransactionType` of the new `Transaction`
    pub fn new(
        timestamp: u64,
        inputs: Vec<OutputIdProto>,
        outputs: Vec<OutputProto>,
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
}