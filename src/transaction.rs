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
        let sig = sign_message(&hash, secret_key);
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




#[cfg(test)]
mod tests {
    use crate::{panda_protos::{OutputProto, OutputIdProto, transaction_proto::TxType}, test_utilities::globals_init::make_timestamp_generator_for_test, keypair::Keypair, crypto::verify_bytes_message};
    use super::Transaction;

    #[tokio::test]
    async fn transaction_signature_test() {
        let timestamp_generator = make_timestamp_generator_for_test();
        let keypair = Keypair::new();
        let output = OutputProto::new(*keypair.get_public_key(), 2);
        let input = OutputIdProto::new([1; 32], 0);
        let tx = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![input],
            vec![output],
            TxType::Seed,
            vec![],
            keypair.get_secret_key(),
        );
        assert!(verify_bytes_message(tx.get_hash(), tx.get_signature(), &keypair.get_public_key().serialize().to_vec()));
    }
}