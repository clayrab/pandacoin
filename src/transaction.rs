use crate::crypto::hash_bytes;
use crate::panda_protos::transaction_proto::TxType;
use crate::panda_protos::TransactionProto;
use crate::panda_protos::{OutputIdProto, OutputProto};
use crate::types::Sha256Hash;
use prost::Message;
use std::convert::TryInto;
use std::io::Cursor;

impl TransactionProto {
    pub fn new(
        inputs: Vec<OutputIdProto>,
        outputs: Vec<OutputProto>,
        txtype: TxType,
        timestamp: u64,
        message: Vec<u8>,
    ) -> Self {
        let mut tx = TransactionProto {
            hash: None,
            timestamp: timestamp,
            inputs: inputs,
            outputs: outputs,
            txtype: txtype as i32,
            message: message,
            signature: vec![],
        };
        tx.hash = tx.generate_hash();
        tx
    }
    fn generate_hash(&self) -> Option<Vec<u8>> {
        assert!(self.hash.is_none());
        Some(hash_bytes(&self.serialize()).to_vec())
    }
    pub fn get_hash(&self) -> Sha256Hash {
        //&self.hash.unwrap().try_into().unwrap()
        (*self.hash.as_ref().unwrap().clone()).try_into().unwrap()
    }
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.reserve(self.encoded_len());
        self.encode(&mut buf).unwrap();
        buf
    }

    pub fn deserialize(buf: &Vec<u8>) -> TransactionProto {
        TransactionProto::decode(&mut Cursor::new(buf)).unwrap()
    }
}
