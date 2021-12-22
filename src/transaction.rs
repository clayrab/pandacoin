use crate::crypto::{hash_bytes, sign_message};
use crate::panda_protos::transaction_proto::TxType;
use crate::panda_protos::TransactionProto;
use crate::panda_protos::{OutputIdProto, OutputProto};
use crate::types::{Secp256k1SignatureCompact, Sha256Hash};
use prost::Message;
use secp256k1::SecretKey;
use std::convert::{TryFrom, TryInto};
use std::io::Cursor;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    hash: Sha256Hash,
    transaction_proto: TransactionProto,
}

impl Transaction {
    pub fn new(
        timestamp: u64, inputs: Vec<OutputIdProto>, outputs: Vec<OutputProto>, txtype: TxType,
        message: Vec<u8>, secret_key: &SecretKey,
    ) -> Self {
        // sig must be set to zeros before generating hash
        let mut transaction_proto = TransactionProto {
            timestamp,
            inputs,
            outputs,
            txtype: txtype as i32,
            message,
            signature: vec![0; 64],
        };
        let hash = hash_bytes(&transaction_proto.serialize());
        let sig = transaction_proto.sign(secret_key);
        transaction_proto.set_signature(sig);
        Transaction {
            hash,
            transaction_proto,
        }
    }
    pub fn from_proto(mut transaction_proto: TransactionProto) -> Self {
        // sig must be set to zeros before generating hash
        let sig = transaction_proto.signature;
        transaction_proto.signature = vec![0; 64];
        let hash = hash_bytes(&transaction_proto.serialize());
        transaction_proto.signature = sig;
        Transaction {
            hash,
            transaction_proto,
        }
    }
    pub fn into_proto(self) -> TransactionProto {
        self.transaction_proto
    }
    pub fn get_hash(&self) -> &Sha256Hash {
        &self.hash
    }
    pub fn get_timestamp(&self) -> u64 {
        self.transaction_proto.timestamp
    }
    pub fn get_inputs(&self) -> &Vec<OutputIdProto> {
        &self.transaction_proto.inputs
    }
    pub fn get_outputs(&self) -> &Vec<OutputProto> {
        &self.transaction_proto.outputs
    }
    pub fn get_txtype(&self) -> TxType {
        self.transaction_proto.txtype.try_into().unwrap()
    }
    pub fn get_message(&self) -> &Vec<u8> {
        &self.transaction_proto.message
    }
    pub fn get_signature(&self) -> &Vec<u8> {
        &self.transaction_proto.signature
    }
    pub fn get_transaction_proto(&self) -> &TransactionProto {
        &self.transaction_proto
    }
}

impl TransactionProto {
    pub fn sign(&self, secret_key: &SecretKey) -> Secp256k1SignatureCompact {
        assert_eq!(self.signature, [0; 64]);
        sign_message(&self.serialize(), secret_key)
    }
    pub fn set_signature(&mut self, signature: Secp256k1SignatureCompact) {
        self.signature = signature.try_into().unwrap();
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

impl TryFrom<i32> for TxType {
    type Error = ();
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == TxType::Normal as i32 => Ok(TxType::Normal),
            x if x == TxType::Seed as i32 => Ok(TxType::Seed),
            x if x == TxType::Service as i32 => Ok(TxType::Service),
            _ => Err(()),
        }
    }
}
