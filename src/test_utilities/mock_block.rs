use crate::{block::RawBlock, types::Sha256Hash, transaction::Transaction};

/// This Mock RawBlock is used for testing Block Fee
#[derive(Debug)]
pub struct MockRawBlockForBlockFee {
    mock_block_id: u32,
    mock_block_fee: u64,
    mock_block_hash: Sha256Hash,
    mock_parent_hash: Sha256Hash,
    timestamp: u64,
    transactions: Vec<Transaction>,
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
            transactions: vec![],
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
    fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}

/// This Mock RawBlock is used for testing the UTXO Set
#[derive(Debug)]
pub struct MockRawBlockForUTXOSet {
    mock_block_id: u32,
    mock_block_hash: Sha256Hash,
    transactions: Vec<Transaction>,
}
impl MockRawBlockForUTXOSet {
    pub fn new(
        mock_block_id: u32,
        mock_block_hash: Sha256Hash,
        transactions: Vec<Transaction>,
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

    fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
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
    transactions: Vec<Transaction>,
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
        MockRawBlockForBlockchain {
            mock_block_id,
            mock_block_fee,
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
    fn get_block_fee(&self) -> u64 {
        self.mock_block_fee
    }
    fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}

/// This Mock RawBlock is used for testing the blockchain Set
#[derive(Debug)]
pub struct MockRawBlockForForkManager {
    transactions: Vec<Transaction>,
}
impl MockRawBlockForForkManager {
    pub fn new() -> Self {
        MockRawBlockForForkManager {
            transactions: vec![],
        }
    }
}

impl RawBlock for MockRawBlockForForkManager {
    fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }
}
