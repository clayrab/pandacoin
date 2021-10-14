use std::convert::TryInto;
use std::io::Cursor;
use std::str::FromStr;

use prost::Message;
use secp256k1::{PublicKey, Signature};
use crate::crypto::hash_bytes;
use crate::keypair_store::KEYPAIRSTORE_GLOBAL;
use crate::timestamp_generator::TIMESTAMP_GENERATOR_GLOBAL;
use crate::types::Sha256Hash;
use crate::panda_protos::{RawBlockProto};
/// This structure is a basic block, it should be 1-to-1 with the physical data which will be serialized
/// and send over the wire and stored on disk.
/// We provide a useless default implementation for the sake of making different mock RawBlocks easier.
/// We would prefer to define only the interface here and the default implementation in mock_block.rs, but
/// Rust does not allow this.
pub trait RawBlock {
    fn get_signature(&self) -> Signature {
        Signature::from_compact(&[0; 64]).unwrap()
    }
    fn get_hash(&self) -> Sha256Hash {
        [0; 32]
    }
    fn get_block_fee(&self) -> u64 {
        0
    }
    fn get_timestamp(&self) -> u64 {
        0
    }
    fn get_creator(&self) -> PublicKey {
       PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072",).unwrap()
    }
    fn get_previous_block_hash(&self) -> Sha256Hash {
        [1; 32]
    }
    fn get_id(&self) -> u32 {
        0
    }
}

impl RawBlockProto {
    /// Creates a new block using the global keypair.
    pub fn new(
        id: u32,
        creator: PublicKey,
        timestamp: u64,
        previous_block_hash: Sha256Hash,
        // transactions: Vec<Transaction>,
    ) -> Self {
        // TODO add transactions
        // TODO generate a real signature
        // TODO calculate the actual block fee
        let signature = Signature::from_compact(&[0; 64]).unwrap();
        let block_fee = 0;
        let mut block = RawBlockProto {
            hash: None,
            block_fee: Some(block_fee),
            id,
            timestamp,
            creator: creator.serialize().to_vec(),
            signature: signature.serialize_compact().to_vec(),
            previous_block_hash: previous_block_hash.to_vec(),
            merkle_root: vec![],
            transactions: vec![],
        };
        block.hash = Some(hash_bytes(&block.serialize()).to_vec());
        block
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
    pub fn create_genesis_block() -> RawBlockProto {
        RawBlockProto::new(0, KEYPAIRSTORE_GLOBAL.get_keypair().get_public_key().clone(), TIMESTAMP_GENERATOR_GLOBAL.get_timestamp(), [0; 32])
    }
}

impl RawBlock for RawBlockProto {

    fn get_signature(&self) -> Signature {
        // TODO Memoize this?
        Signature::from_compact(&self.signature[..]).unwrap()
    }
    
    fn get_hash(&self) -> Sha256Hash {
        // TODO Memoize this? No, I think we will not. I'd rather not need to have a mutable block all the time
        // simply use unwrap() here and let the errors come.
        (*self.hash.as_ref().unwrap().clone()).try_into().unwrap()
    }

    fn get_block_fee(&self) -> u64 {
        // For now we just want to panic if block_fee is not set.
        // I don't think this needs to be properly memoized, but
        // let's just panic here so we can at least enforce consistency.
        self.block_fee.unwrap()
    }

    fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    fn get_creator(&self) -> PublicKey {
       // TODO Memoize this and return as a shared borrow?
       PublicKey::from_slice(&self.creator[..]).unwrap()
    }

    // pub fn transactions(&self) -> &Vec<Transaction> {
    //     &self.core.transactions
    // }

    fn get_previous_block_hash(&self) -> Sha256Hash {
        // TODO Memoize this and return as a shared borrow?
        self.previous_block_hash.clone().try_into().unwrap()

    }

    fn get_id(&self) -> u32 {
        self.id
    }
}
