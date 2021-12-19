use crate::crypto::hash_bytes;
use crate::panda_protos::RawBlockProto;
use crate::transaction::Transaction;
use crate::types::Sha256Hash;
use crate::utxoset::AbstractUtxoSet;
use async_std::sync::RwLock;
use prost::Message;
use secp256k1::{PublicKey, Signature};
use std::convert::TryInto;
use std::fmt::Debug;
use std::io::Cursor;
use std::str::FromStr;
use std::sync::Arc;
/// This structure is a basic block, it should be 1-to-1 with the physical data which will be serialized
/// and send over the wire and stored on disk.
/// We provide a useless default implementation for the sake of making different mock RawBlocks easier.
/// We would prefer to define only the interface here and the default implementation in mock_block.rs, but
/// Rust does not allow this.
///

pub trait RawBlock: Sync + Debug + Send {
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
    fn get_transactions(&self) -> &Vec<Transaction>;
}
#[derive(Debug)]
pub struct PandaBlock {
    hash: Sha256Hash,
    fee: u64,
    transactions: Vec<Transaction>,
    block_proto: RawBlockProto,
}

impl PandaBlock {
    pub async fn new(
        id: u32,
        creator: PublicKey,
        timestamp: u64,
        previous_block_hash: Sha256Hash,
        utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        // transactions: Vec<Transaction>,
    ) -> Self {
        // TODO add transactions
        // TODO generate a real signature
        let signature = Signature::from_compact(&[0; 64]).unwrap();
        let block_proto = RawBlockProto {
            id,
            timestamp,
            creator: creator.serialize().to_vec(),
            signature: signature.serialize_compact().to_vec(),
            previous_block_hash: previous_block_hash.to_vec(),
            merkle_root: vec![],
            transactions: vec![],
        };
        let hash: Sha256Hash = block_proto.generate_hash().try_into().unwrap();
        let utxoset = utxoset_ref.read().await;
        let block_fees = utxoset.block_fees(&block_proto);
        PandaBlock {
            hash,
            fee: block_fees,
            block_proto,
            transactions: vec![],
        }
    }

    pub fn new_genesis_block(
        creator: PublicKey,
        timestamp: u64,
        block_fee: u64,
    ) -> Box<dyn RawBlock> {
        // TODO add transactions
        let signature = Signature::from_compact(&[0; 64]).unwrap();
        let block_proto = RawBlockProto {
            id: 0,
            timestamp,
            creator: creator.serialize().to_vec(),
            signature: signature.serialize_compact().to_vec(),
            previous_block_hash: [0; 32].to_vec(),
            merkle_root: vec![],
            transactions: vec![],
        };
        let hash: Sha256Hash = block_proto.generate_hash().try_into().unwrap();
        let block = PandaBlock {
            hash,
            fee: block_fee,
            block_proto,
            transactions: vec![],
        };
        Box::new(block)
    }
}

impl RawBlockProto {
    pub fn generate_hash(&self) -> Vec<u8> {
        hash_bytes(&self.serialize()).to_vec()
    }
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
        //(*self.hash.as_ref()).try_into().unwrap()
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
        self.block_proto
            .previous_block_hash
            .clone()
            .try_into()
            .unwrap()
    }
    fn get_id(&self) -> u32 {
        self.block_proto.id
    }
    fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}

#[cfg(test)]
mod test {
    // use crate::{
    //     block::{PandaBlock, RawBlock},
    //     test_utilities::globals_init::{
    //         make_keypair_store_for_test, make_timestamp_generator_for_test,
    //     },
    // };

    // #[tokio::test]
    // async fn new_block_fee_test() {
    //     let keypair_store = make_keypair_store_for_test();
    //     let timestamp_generator = make_timestamp_generator_for_test();
    //     let block1 = PandaBlock::new(
    //         0,
    //         *keypair_store.get_keypair().get_public_key(),
    //         timestamp_generator.get_timestamp(),
    //         [0; 32],
    //     );
    //     timestamp_generator.advance(111);
    //     let block2 = PandaBlock::new(
    //         0,
    //         *keypair_store.get_keypair().get_public_key(),
    //         timestamp_generator.get_timestamp(),
    //         [0; 32],
    //     );
    //     assert_eq!(block1.get_timestamp() + 111, block2.get_timestamp());
    // }
}
