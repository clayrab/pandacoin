use std::cmp::Ordering;
use std::cmp::Ord;

use crate::transaction::Transaction;
use crate::types::Sha256Hash;

// insert
// remove
// get
// len()

#[derive(Debug, Eq)]
pub struct MempoolTxSetItem {
    fee: u64,
    hash: Sha256Hash,
}

impl MempoolTxSetItem {
    pub fn new(fee: u64, hash: Sha256Hash) -> Self {
        MempoolTxSetItem {
            fee,
            hash,
        }
    }
}

impl Ord for MempoolTxSetItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fee.cmp(&other.fee)
    }
}

impl PartialOrd for MempoolTxSetItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MempoolTxSetItem {
    fn eq(&self, other: &Self) -> bool {
        self.fee == other.fee
    }
}
