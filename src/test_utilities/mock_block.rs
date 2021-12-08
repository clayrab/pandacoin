use crate::{block::RawBlock, panda_protos::TransactionProto, types::Sha256Hash};

/// This Mock RawBlock is used for testing Block Fee
#[derive(Debug)]
pub struct MockRawBlockForBlockFee {
    block_fee: u64,
    timestamp: u64,
    transactions: Vec<TransactionProto>,
}
impl MockRawBlockForBlockFee {
    pub fn new(block_fee: u64, timestamp: u64) -> Self {
        MockRawBlockForBlockFee {
            block_fee,
            timestamp,
            transactions: vec![],
        }
    }
}
impl RawBlock for MockRawBlockForBlockFee {
    fn get_block_fee(&self) -> u64 {
        self.block_fee
    }
    fn get_timestamp(&self) -> u64 {
        self.timestamp
    }
    fn get_transactions(&self) -> &Vec<TransactionProto> {
        &self.transactions
    }
}

/// This Mock RawBlock is used for testing the UTXO Set
#[derive(Debug)]
pub struct MockRawBlockForUTXOSet {
    mock_block_id: u32,
    mock_block_hash: Sha256Hash,
    transactions: Vec<TransactionProto>,
}
impl MockRawBlockForUTXOSet {
    pub fn new(
        mock_block_id: u32,
        mock_block_hash: Sha256Hash,
        transactions: Vec<TransactionProto>,
    ) -> Self {
        MockRawBlockForUTXOSet {
            mock_block_id,
            mock_block_hash,
            transactions,
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

    fn get_transactions(&self) -> &Vec<TransactionProto> {
        &self.transactions
    }
}

/// This Mock RawBlock is used for testing the blockchain Set
#[derive(Debug)]
pub struct MockRawBlockForBlockchain {
    mock_block_id: u32,
    mock_block_hash: Sha256Hash,
    mock_parent_hash: Sha256Hash,
    timestamp: u64,
    transactions: Vec<TransactionProto>,
}
impl MockRawBlockForBlockchain {
    pub fn new(
        mock_block_id: u32,
        mock_block_hash: Sha256Hash,
        mock_parent_hash: Sha256Hash,
        timestamp: u64,
        transactions: Vec<TransactionProto>,
    ) -> Self {
        MockRawBlockForBlockchain {
            mock_block_id,
            mock_block_hash,
            mock_parent_hash,
            timestamp,
            transactions,
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
    fn get_transactions(&self) -> &Vec<TransactionProto> {
        &self.transactions
    }
}

/// This Mock RawBlock is used for testing the blockchain Set
#[derive(Debug)]
pub struct MockRawBlockForForkManager {
    transactions: Vec<TransactionProto>,
}
impl MockRawBlockForForkManager {
    pub fn new() -> Self {
        MockRawBlockForForkManager {
            transactions: vec![],
        }
    }
}

impl RawBlock for MockRawBlockForForkManager {
    fn get_transactions(&self) -> &Vec<TransactionProto> {
        &self.transactions
    }
}
