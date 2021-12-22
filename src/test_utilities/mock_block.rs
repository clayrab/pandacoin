use crate::{
    block::RawBlock,
    miniblock::MiniBlock,
    panda_protos::{MiniBlockProto, TransactionProto},
    transaction::Transaction,
    types::Sha256Hash,
};

/// This Mock RawBlock is used for testing Block Fee
#[derive(Debug)]
pub struct MockRawBlockForBlockFee {
    mock_block_id: u32,
    mock_block_fee: u64,
    mock_block_hash: Sha256Hash,
    mock_parent_hash: Sha256Hash,
    timestamp: u64,
    mini_blocks: Vec<MiniBlock>,
}
impl MockRawBlockForBlockFee {
    pub fn new(
        mock_block_id: u32,
        mock_block_fee: u64,
        mock_block_hash: Sha256Hash,
        mock_parent_hash: Sha256Hash,
        timestamp: u64,
    ) -> Self {
        MockRawBlockForBlockFee {
            mock_block_id,
            mock_block_fee,
            mock_block_hash,
            mock_parent_hash,
            timestamp,
            mini_blocks: vec![],
        }
    }
}
impl RawBlock for MockRawBlockForBlockFee {
    fn get_id(&self) -> u32 {
        self.mock_block_id
    }
    fn get_block_fee(&self) -> u64 {
        self.mock_block_fee
    }
    fn get_hash(&self) -> &Sha256Hash {
        &self.mock_block_hash
    }
    fn get_previous_block_hash(&self) -> Sha256Hash {
        self.mock_parent_hash
    }
    fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
    fn get_mini_blocks(&self) -> &Vec<MiniBlock> {
        &self.mini_blocks
    }
}

/// This Mock RawBlock is used for testing the UTXO Set
#[derive(Debug)]
pub struct MockRawBlockForUTXOSet {
    mock_block_id: u32,
    mock_block_hash: Sha256Hash,
    mini_blocks: Vec<MiniBlock>,
}
impl MockRawBlockForUTXOSet {
    pub fn new(
        mock_block_id: u32,
        mock_block_hash: Sha256Hash,
        transactions: Vec<Transaction>,
    ) -> Self {
        let mini_block_transactions: Vec<TransactionProto> =
            transactions.into_iter().map(|tx| tx.into_proto()).collect();
        let mini_block_proto = MiniBlockProto {
            receiver: vec![],
            creator: vec![],
            signature: vec![],
            merkle_root: vec![],
            transactions: mini_block_transactions,
        };
        let mini_block = MiniBlock::from_proto(mini_block_proto);
        MockRawBlockForUTXOSet {
            mock_block_id,
            mock_block_hash,
            mini_blocks: vec![mini_block],
        }
    }
}
impl RawBlock for MockRawBlockForUTXOSet {
    fn get_id(&self) -> u32 {
        self.mock_block_id
    }
    fn get_hash(&self) -> &Sha256Hash {
        &self.mock_block_hash
    }
    fn get_mini_blocks(&self) -> &Vec<MiniBlock> {
        &self.mini_blocks
    }
}

/// This Mock RawBlock is used for testing the blockchain Set
#[derive(Debug)]
pub struct MockRawBlockForBlockchain {
    mock_block_id: u32,
    mock_block_fee: u64,
    mock_block_hash: Sha256Hash,
    mock_parent_hash: Sha256Hash,
    timestamp: u64,
    mini_blocks: Vec<MiniBlock>,
}
impl MockRawBlockForBlockchain {
    pub fn new(
        mock_block_id: u32,
        mock_block_fee: u64,
        mock_block_hash: Sha256Hash,
        mock_parent_hash: Sha256Hash,
        timestamp: u64,
        transactions: Vec<Transaction>,
    ) -> Self {
        let mini_block_transactions: Vec<TransactionProto> =
            transactions.into_iter().map(|tx| tx.into_proto()).collect();
        let mini_block_proto = MiniBlockProto {
            receiver: vec![],
            creator: vec![],
            signature: vec![],
            merkle_root: vec![],
            transactions: mini_block_transactions,
        };
        let mini_block = MiniBlock::from_proto(mini_block_proto);

        MockRawBlockForBlockchain {
            mock_block_id,
            mock_block_fee,
            mock_block_hash,
            mock_parent_hash,
            timestamp,
            mini_blocks: vec![mini_block],
        }
    }
}
impl RawBlock for MockRawBlockForBlockchain {
    fn get_id(&self) -> u32 {
        self.mock_block_id
    }
    fn get_hash(&self) -> &Sha256Hash {
        &self.mock_block_hash
    }
    fn get_previous_block_hash(&self) -> Sha256Hash {
        self.mock_parent_hash
    }
    fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
    fn get_block_fee(&self) -> u64 {
        self.mock_block_fee
    }
    fn get_mini_blocks(&self) -> &Vec<MiniBlock> {
        &self.mini_blocks
    }
}

/// This Mock RawBlock is used for testing the blockchain Set
#[derive(Debug)]
pub struct MockRawBlockForForkManager {
    mini_blocks: Vec<MiniBlock>,
}
impl MockRawBlockForForkManager {
    pub fn new() -> Self {
        MockRawBlockForForkManager {
            mini_blocks: vec![],
        }
    }
}

impl RawBlock for MockRawBlockForForkManager {
    // fn get_transactions(&self) -> &Vec<Transaction> {
    //     &self.transactions
    // }
    fn get_mini_blocks(&self) -> &Vec<MiniBlock> {
        &self.mini_blocks
    }
}
