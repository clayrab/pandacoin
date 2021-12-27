use std::io::Cursor;

use prost::Message;
use secp256k1::{PublicKey, Signature};

use crate::{
    crypto::hash_bytes,
    panda_protos::{MiniBlockProto, TransactionProto},
    transaction::Transaction,
    types::Sha256Hash,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiniBlock {
    hash: Sha256Hash,
    fee: u64,
    transactions: Vec<Transaction>,
    mini_block_proto: MiniBlockProto,
}

impl MiniBlock {
    pub fn from_serialiazed_proto(serialized_mini_block_proto: Vec<u8>) -> Self {
        let hash = hash_bytes(&serialized_mini_block_proto);
        let mut mini_block_proto = MiniBlockProto::deserialize(serialized_mini_block_proto);
        // "partial move" the transaction out of the proto and into "transactions", transofmr them from TransactionProto
        // into Transaction
        let transactions: Vec<Transaction> = mini_block_proto
            .transactions
            .into_iter()
            .map(Transaction::from_proto)
            .collect();
        // Set the proto transactions to an empty vector to avoid ownership issues.
        mini_block_proto.transactions = vec![];
        // Now miniblock can own everything, but transactions have been transformed from Protos into Transaction
        // objects and hopefully we've avoided any heavy memory operations...
        MiniBlock {
            hash,
            fee: 0,
            transactions,
            mini_block_proto,
        }
    }
    pub fn from_proto(mut mini_block_proto: MiniBlockProto) -> Self {
        // TODO This is really bad. We are re-serializing this mini-block just to get the hash, but we just deserialized it as part of the
        // full block and could easily just use the raw bytes. We need to optimized this. For now we will leave it though.
        // The function above, new_from_serialiazed_proto, is also wrong because it is deserializing, which is alreayd happening when we
        // deserialize the block. We want to let the Proto lib deserialize but then hash the raw bytes that are in the serialized.

        let hash = hash_bytes(&mini_block_proto.serialize());
        let transactions: Vec<Transaction> = mini_block_proto
            .transactions
            .into_iter()
            .map(Transaction::from_proto)
            .collect();
        // Set the proto transactions to an empty vector to avoid ownership issues.
        mini_block_proto.transactions = vec![];
        MiniBlock {
            hash,
            fee: 0,
            transactions,
            mini_block_proto,
        }
    }
    pub fn into_proto(mut self) -> MiniBlockProto {
        let transactions: Vec<TransactionProto> = self
            .transactions
            .into_iter()
            .map(|tx_proto| tx_proto.into_proto())
            .collect();
        self.mini_block_proto.transactions = transactions;
        self.mini_block_proto
    }
    pub fn get_hash(&self) -> &Sha256Hash {
        &self.hash
    }
    pub fn get_fee(&self) -> u64 {
        self.fee
    }
    pub fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
    pub fn get_mini_block_proto(&self) -> &MiniBlockProto {
        &self.mini_block_proto
    }
    pub fn get_receiver(&self) -> PublicKey {
        PublicKey::from_slice(&self.mini_block_proto.receiver[..]).unwrap()
    }
    pub fn get_creator(&self) -> PublicKey {
        PublicKey::from_slice(&self.mini_block_proto.creator[..]).unwrap()
    }
    pub fn get_signature(&self) -> Signature {
        Signature::from_compact(&self.mini_block_proto.signature[..]).unwrap()
    }
}

impl MiniBlockProto {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.reserve(self.encoded_len());
        self.encode(&mut buf).unwrap();
        buf
    }
    pub fn deserialize(buf: Vec<u8>) -> MiniBlockProto {
        MiniBlockProto::decode(&mut Cursor::new(buf)).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::MiniBlock;
    use crate::{
        crypto::sign_message,
        keypair::Keypair,
        panda_protos::{transaction_proto::TxType, MiniBlockProto, TransactionProto},
    };

    #[tokio::test]
    async fn mini_block_test() {
        let keypair_1 = Keypair::new();
        let keypair_2 = Keypair::new();
        let signature = sign_message(&[0; 32], keypair_2.get_secret_key());
        let transaction_proto = TransactionProto {
            timestamp: 12345,
            inputs: vec![],
            outputs: vec![],
            txtype: TxType::Normal as i32,
            message: vec![],
            signature: vec![],
        };
        let mini_block_proto = MiniBlockProto {
            receiver: keypair_1.get_public_key().serialize().to_vec(),
            creator: keypair_2.get_public_key().serialize().to_vec(),
            signature: signature.to_vec(),
            transactions: vec![transaction_proto],
        };
        let serialized_mini_block_proto = mini_block_proto.serialize();
        let mini_block = MiniBlock::from_serialiazed_proto(serialized_mini_block_proto.clone());
        let deserialized_mini_block_proto =
            MiniBlockProto::deserialize(serialized_mini_block_proto);

        assert_eq!(mini_block_proto, deserialized_mini_block_proto);

        assert_eq!(
            mini_block.get_receiver().serialize().to_vec(),
            mini_block_proto.receiver
        );
        assert_eq!(
            mini_block.get_receiver().serialize().to_vec(),
            mini_block_proto.receiver
        );
        assert_eq!(
            mini_block.get_creator().serialize().to_vec(),
            mini_block_proto.creator
        );
        assert_eq!(
            mini_block.get_signature().serialize_compact().to_vec(),
            mini_block_proto.signature
        );
        assert_eq!(
            mini_block.get_transactions().len(),
            mini_block_proto.transactions.len()
        );
        //assert_eq!(mini_block, mini_block_2);
        let tx = mini_block.get_transactions().get(0).unwrap();
        let tx_proto = mini_block_proto.transactions.get(0).unwrap();
        assert_eq!(tx.get_timestamp(), tx_proto.timestamp);
        assert_eq!(tx.get_txtype() as i32, tx_proto.txtype);

        let mini_block_2 = MiniBlock::from_proto(deserialized_mini_block_proto);
        assert_eq!(
            mini_block_2.get_receiver().serialize().to_vec(),
            mini_block_proto.receiver
        );
        assert_eq!(
            mini_block_2.get_creator().serialize().to_vec(),
            mini_block_proto.creator
        );
        assert_eq!(
            mini_block_2.get_signature().serialize_compact().to_vec(),
            mini_block_proto.signature
        );
        assert_eq!(
            mini_block_2.get_transactions().len(),
            mini_block_proto.transactions.len()
        );
        let tx_2 = mini_block_2.get_transactions().get(0).unwrap();
        assert_eq!(tx_2.get_timestamp(), tx_proto.timestamp);
        assert_eq!(tx_2.get_txtype() as i32, tx_proto.txtype);
    }
}
