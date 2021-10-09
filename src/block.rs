use std::convert::TryInto;

use secp256k1::{PublicKey, Signature};

use crate::types::Sha256Hash;
use crate::panda_protos::{RawBlockProto};
/// This structure is a basic block, it should be 1-to-1 with the physical data which will be serialized
/// and send over the wire and stored on disk.
impl RawBlockProto {
    /// Creates a new block using the global keypair.
    pub fn new(
        id: u32,
        creator: PublicKey,
        timestamp: u64,
        previous_block_hash: Sha256Hash,
        // transactions: Vec<Transaction>,
    ) -> Self {
        // let hash = hash_bytes(&self.serialize());
        let signature = Signature::from_compact(&[0; 64]).unwrap();

        RawBlockProto {
            hash: None,
            block_fee: None,
            id,
            timestamp,
            creator: creator.serialize().to_vec(),
            signature: signature.serialize_compact().to_vec(),
            previous_block_hash: previous_block_hash.to_vec(),
            merkle_root: vec![],
            transactions: vec![],
        }
    }

    pub fn get_signature(&self) -> Signature {
        // TODO Memoize this and return as a shared borrow?
        Signature::from_compact(&self.signature[..]).unwrap()
    }
    
    pub fn get_hash(&self) -> Sha256Hash {
        // TODO Memoize this and return as a shared borrow?
        (*self.hash.as_ref().unwrap().clone()).try_into().unwrap()
    }

    pub fn get_block_fee(&self) -> u64 {
        // For now we just want to panic if block_fee is not set.
        // I don't think this needs to be properly memoized, but
        // let's just panic here so we can at least enforce consistency.
        self.block_fee.unwrap()
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn get_creator(&self) -> PublicKey {
       // TODO Memoize this and return as a shared borrow?
       PublicKey::from_slice(&self.creator[..]).unwrap()
    }

    // pub fn transactions(&self) -> &Vec<Transaction> {
    //     &self.core.transactions
    // }

    pub fn get_previous_block_hash(&self) -> Sha256Hash {
        // TODO Memoize this and return as a shared borrow?
        self.previous_block_hash.clone().try_into().unwrap()

    }

    pub fn get_id(&self) -> u32 {
        self.id
    }
}