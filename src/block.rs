use secp256k1::{PublicKey, Signature};

use crate::{crypto::hash_bytes, types::Sha256Hash};


/// This structure is a basic block, it should be 1-to-1 with the physical data which will be serialized
/// and send over the wire and stored on disk.
pub struct RawBlock {
    // Memoized hash of the block
    // hash: Sha256Hash,
    // Memoized block fee
    //block_fee: u64,
    /// `Publickey` of the block creator
    creator: PublicKey,
    /// `secp256k1::Signature` verifying the cretor of the Block's data
    signature: Signature,
    /// Block id
    id: u64,
    /// Block timestamp
    timestamp: u64,
    /// Byte array hash of the previous block in the chain
    previous_block_hash: Sha256Hash,
    // tranasctions will be replaced by a merkle tree and may be placed in a separate data structure...
    // transactions: Vec<Transaction>,
}

impl RawBlock {
    /// Creates a new block using the global keypair. If you wish to deserialize a block from
    /// 
    pub fn new(
        // block_fee: u64,
        id: u64,
        creator: PublicKey,
        signature: Signature,
        timestamp: u64,
        previous_block_hash: Sha256Hash,
        treasury: u64,
        // transactions: Vec<Transaction>,
    ) -> Self {
        // let hash = hash_bytes(&self.serialize());
        let signature = Signature::from_compact(&[0; 64]).unwrap();
        RawBlock {
            // hash,
            // block_fee,
            creator,
            signature,
            id,
            timestamp,
            previous_block_hash,
            //transactions: transactions,
        }
    }

    /// Serialize a Block for transport or disk.
    /// [len of transactions - 4 bytes - u32]
    /// [id - 8 bytes - u64]
    /// [timestamp - 8 bytes - u64]
    /// [previous_block_hash - 32 bytes - SHA 256 hash]
    /// [creator - 33 bytes - Secp25k1 pubkey compact format]
    /// [merkle_root - 32 bytes - SHA 256 hash
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [treasury - 8 bytes - u64]
    /// [burnfee - 8 bytes - u64]
    /// [difficulty - 8 bytes - u64]
    /// [transaction][transaction][transaction]...
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![];
        //buf.extend((self.transactions.iter().len() as u32).to_be_bytes());
        buf.extend(&self.id.to_be_bytes());
        buf.extend(&self.timestamp.to_be_bytes());
        buf.extend(&self.previous_block_hash);
        buf.extend(&self.creator.serialize());
        //buf.extend(&self.core.signature);
        //let mut serialized_txs_buf = vec![];
        // self.transactions.iter().for_each(|transaction| {
        //     serialized_txs_buf.extend(transaction.serialize());
        // });
        // buf.extend(serialized_txs_buf);
        buf
    }
    /// Deserialize from bytes to a Block.
    /// [len of transactions - 4 bytes - u32]
    /// [id - 8 bytes - u64]
    /// [timestamp - 8 bytes - u64]
    /// [previous_block_hash - 32 bytes - SHA 256 hash]
    /// [creator - 33 bytes - Secp25k1 pubkey compact format]
    /// [merkle_root - 32 bytes - SHA 256 hash
    /// [signature - 64 bytes - Secp25k1 sig]
    /// [treasury - 8 bytes - u64]
    /// [burnfee - 8 bytes - u64]
    /// [difficulty - 8 bytes - u64]
    /// [transaction][transaction][transaction]...
    // pub fn deserialize(bytes: &Vec<u8>) -> RawBlock {
    //     let transactions_len: u32 = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
    //     let id: u64 = u64::from_be_bytes(bytes[4..12].try_into().unwrap());
    //     let timestamp: u64 = u64::from_be_bytes(bytes[12..20].try_into().unwrap());
    //     let previous_block_hash: Sha256Hash = bytes[20..52].try_into().unwrap();
    //     let creator: PublicKey = PublicKey::from_slice(&bytes[52..85]).unwrap();
    //     // let signature: SaitoSignature = bytes[117..181].try_into().unwrap();

    //     let treasury: u64 = u64::from_be_bytes(bytes[85..93].try_into().unwrap());
    //     let burnfee: u64 = u64::from_be_bytes(bytes[93..101].try_into().unwrap());
    //     let mut transactions = vec![];
    //     let mut start_of_transaction_data = 101;
    //     for _n in 0..transactions_len {
    //         let transaction_data_len: usize = u32::from_be_bytes(
    //             bytes[start_of_transaction_data..start_of_transaction_data + 4]
    //                 .try_into()
    //                 .unwrap(),
    //         ) as usize;
    //         let end_of_transaction_data = start_of_transaction_data + transaction_data_len + 68;
    //         let transaction = Transaction::deserialize(
    //             &bytes[start_of_transaction_data..end_of_transaction_data].to_vec(),
    //         );
    //         transactions.push(transaction);
    //         start_of_transaction_data = end_of_transaction_data;
    //     }
    //     RawBlock::new(
    //         id,
    //         timestamp,
    //         previous_block_hash,
    //         creator,
    //         treasury,
    //         burnfee,
    //         transactions,
    //     )
    // }
}