// use std::collections::HashSet;
// use crate::types::Sha256Hash;

// /// A set of Outputs can be determined from the transactions alone.
// /// This datastructure is used to communicate the state of node's 
// /// mempools.
// #[derive(Debug)]
// pub struct TransactionSet {
//     tx_hash_set: HashSet<Sha256Hash>,
// }

// impl TransactionSet {
//     /// Create TransactionSet
//     pub fn new() -> Self {
//         TransactionSet {
//             tx_hash_set: HashSet::new(),
//         }  
//     }
//     /// Create a TransactionSet from a set of hashes
//     pub fn from(tx_hash_set: HashSet<Sha256Hash>) -> Self {
//         TransactionSet {
//             tx_hash_set: tx_hash_set,
//         }
//     }

//     pub fn insert(&mut self, tx_hash: Sha256Hash) -> bool {
//         self.tx_hash_set.insert(tx_hash)
//     }

//     pub fn remove(&mut self, tx_hash: &Sha256Hash) -> bool {
//         self.tx_hash_set.remove(tx_hash)
//     }

//     pub fn get(&self, tx_hash: &Sha256Hash) -> Option<&Sha256Hash> {
//         self.tx_hash_set.get(tx_hash)
//     }

//     pub fn len(&self) -> usize {
//         self.tx_hash_set.len()
//     }

//     pub fn serialize(&self) {

//     }

//     pub fn deserialize() -> Self {
//         TransactionSet {
//             tx_hash_set: HashSet::new(),
//         }
//     }
// }
