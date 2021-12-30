use crate::crypto::hash_bytes;
use crate::merkle_tree_manager::MerkleTree;
use crate::miniblock::MiniBlock;
use crate::panda_protos::{MiniBlockProto, RawBlockProto};
use crate::transaction::Transaction;
use crate::types::Sha256Hash;
use crate::utxoset::AbstractUtxoSet;
use prost::Message;
use secp256k1::{PublicKey, Signature};
use std::convert::TryInto;
use std::fmt::Debug;
use std::io::Cursor;
use std::str::FromStr;
///
/// This structure is a basic block, it should be 1-to-1 with the physical data which will be serialized
/// and send over the wire and stored on disk.
/// We provide a useless default implementation for the sake of making different mock RawBlocks easier.
/// We would prefer to define only the interface here and the default implementation in mock_block.rs, but
/// Rust does not allow this.
///
pub trait RawBlock: Debug + Send + Sync {
    fn get_signature(&self) -> Signature {
        Signature::from_compact(&[0; 64]).unwrap()
    }
    fn get_hash(&self) -> &Sha256Hash {
        &[0; 32]
    }
    fn get_block_fee(&self) -> u64 {
        0
    }
    fn get_timestamp(&self) -> u64 {
        0
    }
    fn get_creator(&self) -> PublicKey {
        PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072")
            .unwrap()
    }
    fn get_previous_block_hash(&self) -> Sha256Hash {
        [1; 32]
    }
    fn get_id(&self) -> u32 {
        0
    }
    fn get_merkle_root(&self) -> Sha256Hash {
        [2; 32]
    }
    fn get_merkle_tree(&self) -> &MerkleTree;
    fn get_mini_blocks(&self) -> &Vec<MiniBlock>;
    fn transactions_iter(&self) -> TransactionsIter<'_> {
        TransactionsIter {
            index: 0,
            mini_block_index: 0,
            mini_blocks: self.get_mini_blocks(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PandaBlock {
    hash: Sha256Hash,
    fee: u64,
    merkle_tree: MerkleTree,
    mini_blocks: Vec<MiniBlock>,
    block_proto: RawBlockProto,
}

impl PandaBlock {
    pub async fn from_serialized_proto(
        serialized_block_proto: Vec<u8>,
        utxoset: &Box<dyn AbstractUtxoSet + Send + Sync>,
    ) -> Self {
        let hash = hash_bytes(&serialized_block_proto);
        let mut block_proto = RawBlockProto::deserialize(&serialized_block_proto);

        // use "partial move" to move the mini-blocks out of the proto and into the Block as full MiniBlocks
        let mini_blocks: Vec<MiniBlock> = block_proto
            .mini_blocks
            .into_iter()
            .map(MiniBlock::from_proto)
            .collect();
        // Set the proto transactions to an empty vector to avoid ownership issues.
        block_proto.mini_blocks = vec![];
        // make a merkle tree from the tx in the mini blocks
        let merkle_tree = MerkleTree::new(TransactionsIter::new(&mini_blocks));
        let fee = TransactionsIter::new(&mini_blocks)
            .map(|tx| utxoset.transaction_fees(tx.get_transaction_proto()))
            .reduce(|tx_fees_a, tx_fees_b| tx_fees_a + tx_fees_b)
            .unwrap();
        PandaBlock {
            hash,
            fee,
            block_proto,
            merkle_tree,
            mini_blocks,
        }
    }

    pub fn new_genesis_block(
        creator: PublicKey,
        timestamp: u64,
        block_fee: u64,
    ) -> Box<dyn RawBlock> {
        // TODO add Seed Tx here.
        let signature = Signature::from_compact(&[0; 64]).unwrap();
        let block_proto = RawBlockProto {
            id: 0,
            timestamp,
            creator: creator.serialize().to_vec(),
            signature: signature.serialize_compact().to_vec(),
            previous_block_hash: [0; 32].to_vec(),
            merkle_root: vec![],
            mini_blocks: vec![],
        };
        let hash = hash_bytes(&block_proto.serialize());
        // make a merkle tree from the tx in the mini blocks
        let merkle_tree = MerkleTree::new(TransactionsIter::new(&vec![]));

        let block = PandaBlock {
            hash,
            fee: block_fee,
            block_proto,
            merkle_tree,
            mini_blocks: vec![],
        };
        Box::new(block)
    }

    pub fn into_proto(mut self) -> RawBlockProto {
        let mini_blocks: Vec<MiniBlockProto> = self
            .mini_blocks
            .into_iter()
            .map(|mini_block| mini_block.into_proto())
            .collect();
        self.block_proto.mini_blocks = mini_blocks;
        self.block_proto
    }
}

impl RawBlockProto {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.reserve(self.encoded_len());
        self.encode(&mut buf).unwrap();
        buf
    }
    pub fn deserialize(buf: &Vec<u8>) -> RawBlockProto {
        RawBlockProto::decode(&mut Cursor::new(buf)).unwrap()
    }
}

impl RawBlock for PandaBlock {
    fn get_signature(&self) -> Signature {
        // TODO Memoize this?
        Signature::from_compact(&self.block_proto.signature[..]).unwrap()
    }
    fn get_hash(&self) -> &Sha256Hash {
        &self.hash
    }
    fn get_block_fee(&self) -> u64 {
        self.fee
    }
    fn get_timestamp(&self) -> u64 {
        self.block_proto.timestamp
    }

    fn get_creator(&self) -> PublicKey {
        // TODO Memoize this?
        PublicKey::from_slice(&self.block_proto.creator[..]).unwrap()
    }
    fn get_previous_block_hash(&self) -> Sha256Hash {
        // TODO Memoize this and return as a shared borrow?
        //      Or, change the return type to Vec<u8>?
        self.block_proto
            .previous_block_hash
            .clone()
            .try_into()
            .unwrap()
    }
    fn get_merkle_root(&self) -> Sha256Hash {
        self.block_proto.merkle_root.clone().try_into().unwrap()
    }
    fn get_merkle_tree(&self) -> &MerkleTree {
        &self.merkle_tree
    }
    fn get_id(&self) -> u32 {
        self.block_proto.id
    }
    fn get_mini_blocks(&self) -> &Vec<MiniBlock> {
        &self.mini_blocks
    }
}

pub struct TransactionsIter<'a> {
    index: usize,
    mini_block_index: usize,
    mini_blocks: &'a Vec<MiniBlock>,
}

impl<'a> TransactionsIter<'a> {
    pub fn new(mini_blocks: &'a Vec<MiniBlock>) -> Self {
        TransactionsIter {
            index: 0,
            mini_block_index: 0,
            mini_blocks,
        }
    }
    pub fn len(&self) -> usize {
        let mut ret_len = 0;
        for mini_block in self.mini_blocks {
            ret_len += mini_block.get_transactions().len();
        }
        ret_len
    }
}

impl<'a> Iterator for TransactionsIter<'a> {
    type Item = &'a Transaction;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        let mut ret_transaction = None;
        while ret_transaction.is_none() {
            match self.mini_blocks.get(self.mini_block_index) {
                Some(mini_block) => match mini_block.get_transactions().get(self.index - 1) {
                    Some(transaction) => {
                        ret_transaction = Some(transaction);
                    }
                    None => {
                        self.index = 0;
                        self.mini_block_index += 1;
                    }
                },
                None => {
                    return None;
                }
            }
        }
        ret_transaction
    }
}

#[cfg(test)]
mod test {
    use crate::{
        block::RawBlock,
        crypto::sign_message,
        keypair::Keypair,
        miniblock::MiniBlock,
        panda_protos::{
            transaction_proto::TxType, MiniBlockProto, RawBlockProto, TransactionProto,
        },
        test_utilities::mock_utxoset::MockUtxoSet,
        utxoset::AbstractUtxoSet,
    };

    use super::PandaBlock;

    #[tokio::test]
    async fn mini_block_test() {
        let utxoset: Box<dyn AbstractUtxoSet + Send + Sync> = Box::new(MockUtxoSet::new());
        let keypair_1 = Keypair::new();
        let keypair_2 = Keypair::new();
        let signature = sign_message(&[0; 32], keypair_2.get_secret_key());
        let transaction_proto_1 = TransactionProto {
            timestamp: 12345,
            inputs: vec![],
            outputs: vec![],
            txtype: TxType::Normal as i32,
            message: vec![],
            signature: vec![],
        };
        let transaction_proto_2 = TransactionProto {
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
            transactions: vec![transaction_proto_1, transaction_proto_2],
        };
        let mini_block = MiniBlock::from_proto(mini_block_proto.clone());
        let block_proto = RawBlockProto {
            id: 123,
            timestamp: 1234,
            creator: keypair_1.get_public_key().serialize().to_vec(),
            signature: signature.to_vec(),
            previous_block_hash: [0; 32].to_vec(),
            merkle_root: vec![],
            mini_blocks: vec![mini_block_proto],
        };
        let block = PandaBlock::from_serialized_proto(block_proto.serialize(), &utxoset).await;
        let block_proto_again = block.clone().into_proto();
        assert_eq!(block_proto, block_proto_again);
        let block_again =
            PandaBlock::from_serialized_proto(block_proto_again.serialize(), &utxoset).await;
        assert_eq!(block.get_hash(), block_again.get_hash());
        assert_eq!(block.get_mini_blocks()[0], mini_block);
    }
}
