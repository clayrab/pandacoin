use crate::constants::STARTING_BLOCK_FEE;
use crate::crypto::hash_bytes;
use crate::keypair_store::KEYPAIRSTORE_GLOBAL;
use crate::panda_protos::{RawBlockProto, TransactionProto};
use crate::timestamp_generator::TIMESTAMP_GENERATOR_GLOBAL;
use crate::types::Sha256Hash;
use std::fmt::Debug;
use prost::Message;
use secp256k1::{PublicKey, Signature};
use std::convert::TryInto;
use std::io::Cursor;
use std::str::FromStr;
/// This structure is a basic block, it should be 1-to-1 with the physical data which will be serialized
/// and send over the wire and stored on disk.
/// We provide a useless default implementation for the sake of making different mock RawBlocks easier.
/// We would prefer to define only the interface here and the default implementation in mock_block.rs, but
/// Rust does not allow this.
pub trait RawBlock: Sync + Debug + Send {
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
        PublicKey::from_str("0225ee90fc71570613b42e29912a760bb0b2da9182b2a4271af9541b7c5e278072")
            .unwrap()
    }
    fn get_previous_block_hash(&self) -> Sha256Hash {
        [1; 32]
    }
    fn get_id(&self) -> u32 {
        0
    }
    fn get_transactions(&self) -> &Vec<TransactionProto> ;
}

impl RawBlockProto {
    /// Creates a new block using the global keypair and timestamp generator.
    pub fn new(
        id: u32,
        creator: PublicKey,
        previous_block_hash: Sha256Hash,
        // transactions: Vec<Transaction>,
    ) -> Self {
        // TODO add transactions
        // TODO generate a real signature
        // TODO calculate the actual block fee
        let signature = Signature::from_compact(&[0; 64]).unwrap();
        let timestamp = TIMESTAMP_GENERATOR_GLOBAL.get().unwrap().get_timestamp();
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
        block.hash = block.generate_hash();
        block
    }

    pub fn generate_hash(&self) -> Option<Vec<u8>> {
        assert!(self.hash.is_none());
        Some(hash_bytes(&self.serialize()).to_vec())
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
        let mut block = RawBlockProto::new(
            0,
            KEYPAIRSTORE_GLOBAL
                .get()
                .unwrap()
                .get_keypair()
                .get_public_key()
                .clone(),
            [0; 32],
        );
        block.block_fee = Some(STARTING_BLOCK_FEE);
        block
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
        // TODO Memoize this?
        PublicKey::from_slice(&self.creator[..]).unwrap()
    }

    fn get_transactions(&self) -> &Vec<TransactionProto> {
        &self.transactions
    }

    fn get_previous_block_hash(&self) -> Sha256Hash {
        // TODO Memoize this and return as a shared borrow?
        self.previous_block_hash.clone().try_into().unwrap()
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

#[cfg(test)]
mod test {
    use crate::{
        block::RawBlock, keypair_store::KEYPAIRSTORE_GLOBAL, panda_protos::RawBlockProto,
        test_utilities::init_globals_for_tests, timestamp_generator::TIMESTAMP_GENERATOR_GLOBAL,
    };

    #[tokio::test]
    async fn new_block_fee_test() {
        init_globals_for_tests();
        let block1 = RawBlockProto::new(
            0,
            KEYPAIRSTORE_GLOBAL
                .get()
                .unwrap()
                .get_keypair()
                .get_public_key()
                .clone(),
            [0; 32],
        );
        TIMESTAMP_GENERATOR_GLOBAL.get().unwrap().advance(111);
        let block2 = RawBlockProto::new(
            0,
            KEYPAIRSTORE_GLOBAL
                .get()
                .unwrap()
                .get_keypair()
                .get_public_key()
                .clone(),
            [0; 32],
        );
        assert_eq!(block1.get_timestamp() + 111, block2.get_timestamp());
    }
}
