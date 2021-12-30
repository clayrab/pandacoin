use crate::{crypto::hash_bytes, transaction::Transaction, types::Sha256Hash};
// needs to build merkle trees for blocks and validate the merkle root.
// needs to be able to take a pubkey and produce a "lite block".
//

// transaction hash is a proxy for unspent outputs because when used as an
// input we are only interested in the validity of the transaction wherein
// the output was created(by hash)

// We can assume that the "lite block service" will keep a map of pubkey -> tranasction. There is no way around this, we
// need some kind of pubkey-indexed data structure. To save a lot of space, we can base this on ordinals rather than hashes.
// So, we will also based the merkle trees on the transaction ordinals and assume that

pub struct MerkleTreeSubTree {
    pub index: usize,
    pub side_hashes: Vec<Sha256Hash>,
}
#[derive(Clone, Debug)]
pub struct MerkleTree {
    size: usize,
    leafs: Vec<Sha256Hash>,
    nodes: Vec<Option<Sha256Hash>>,
}
/// An example of inner node indexes:
///```text
///                  8
///                /   \
///               /     \
///             4        12
///            / \       / \
///           /   \     /   \
///          2    6    10    14
///         /\    /\   /\    /\
///        /  \  /  \ /  \  /  \
///        1  3  5  7 9 11 13 15 <- we don't store these, they resolve to the tx hashes below
///        |  |  |  | |  |  |  |
///       T0 T1 T2 T3 T4 T5 T6 T7 <- leafs are transaction hashes
/// ```
impl MerkleTree {
    pub fn new<'a, I>(iterator: I) -> Self
    where
        I: Iterator<Item = &'a Transaction>,
    {
        let mut num_items: usize = 0;
        let mut leafs: Vec<Sha256Hash> = vec![];

        for transaction in iterator {
            num_items += 1;
            leafs.push(*transaction.get_hash());
        }
        let mut size = MerkleTree::next_power_of_2(num_items);
        for _i in 1..size - num_items {
            leafs.push([0; 32]);
        }
        if size == 0 {
            size = 1;
        }
        let mut nodes: Vec<Option<Sha256Hash>> = vec![None; size];
        nodes[0] = Some([0; 32]);
        let mut starting_index = 1;
        let mut index = starting_index;
        while starting_index < size {
            while index < size {
                let node_hash = MerkleTree::compute_node_hash_at_index(&mut nodes, &leafs, index);
                nodes[index] = Some(node_hash);
                index += 2 * starting_index;
            }
            starting_index *= 2;
            index = starting_index;
        }
        println!("MerkleTree new {}", size);
        MerkleTree { size, leafs, nodes }
    }

    pub fn compute_node_hash_at_index(
        nodes: &Vec<Option<Sha256Hash>>,
        leafs: &Vec<Sha256Hash>,
        index: usize,
    ) -> Sha256Hash {
        if nodes[index].is_some() {
            return nodes[index].unwrap();
        }
        if (index + 1) % 2 == 0 {
            leafs[(index - 1) >> 1]
        } else {
            let mut next_height = 2;
            while (index - next_height) % (next_height << 1) != 0 {
                next_height <<= 1;
            }
            let hash_left =
                MerkleTree::compute_node_hash_at_index(nodes, leafs, index - (next_height >> 1));
            let hash_right =
                MerkleTree::compute_node_hash_at_index(nodes, leafs, index + (next_height >> 1));
            hash_bytes(&[hash_left, hash_right].concat())
        }
    }
    pub fn get_nodes(&self) -> &[Option<Sha256Hash>] {
        &self.nodes
    }
    pub fn get_root(&self) -> &Option<Sha256Hash> {
        &self.nodes[self.size >> 1]
    }
    pub fn make_sub_tree_for_index(&self, index: usize) -> MerkleTreeSubTree {
        let mut side_hashes = vec![];
        let mut center = self.size >> 1;
        let mut range = center;

        while center % 2 == 0 {
            range >>= 1;
            if index > center {
                side_hashes.push(MerkleTree::compute_node_hash_at_index(
                    &self.nodes,
                    &self.leafs,
                    center - range,
                ));
                center += range;
            } else {
                side_hashes.push(MerkleTree::compute_node_hash_at_index(
                    &self.nodes,
                    &self.leafs,
                    center + range,
                ));
                center -= range;
            }
        }
        MerkleTreeSubTree { index, side_hashes }
    }
    pub fn validate_sub_tree(&self, sub_tree: MerkleTreeSubTree) -> bool {
        let mut ret_val = true;
        let mut center = self.size >> 1;
        let mut range = center;
        let mut hashes_iter = sub_tree.side_hashes.iter();
        while center % 2 == 0 {
            range >>= 1;
            if sub_tree.index > center {
                ret_val = ret_val
                    && (&self.nodes[center - range].unwrap() == hashes_iter.next().unwrap());
                center += range;
            } else {
                ret_val = ret_val
                    && (&self.nodes[center + range].unwrap() == hashes_iter.next().unwrap());
                center -= range;
            }
        }
        ret_val
    }
    pub fn next_power_of_2(mut size: usize) -> usize {
        if size == 0 {
            return 0;
        }
        size -= 1;
        let mut ret_val = 1;
        while size > 0 {
            size >>= 1;
            ret_val <<= 1;
        }
        ret_val
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        block::RawBlock,
        crypto::hash_bytes,
        keypair::Keypair,
        merkle_tree_manager::MerkleTree,
        panda_protos::transaction_proto::TxType,
        test_utilities::{
            globals_init::make_timestamp_generator_for_test, mock_block::MockRawBlockForBlockchain,
        },
        transaction::Transaction,
        types::Sha256Hash,
    };

    #[tokio::test]
    async fn next_power_of_2_test() {
        assert_eq!(MerkleTree::next_power_of_2(0), 0);
        assert_eq!(MerkleTree::next_power_of_2(1), 1);
        assert_eq!(MerkleTree::next_power_of_2(2), 2);
        assert_eq!(MerkleTree::next_power_of_2(3), 4);
        assert_eq!(MerkleTree::next_power_of_2(4), 4);
        assert_eq!(MerkleTree::next_power_of_2(5), 8);
        assert_eq!(MerkleTree::next_power_of_2(6), 8);
        assert_eq!(MerkleTree::next_power_of_2(7), 8);
        assert_eq!(MerkleTree::next_power_of_2(8), 8);
        assert_eq!(MerkleTree::next_power_of_2(9), 16);
        assert_eq!(MerkleTree::next_power_of_2(16), 16);
        assert_eq!(MerkleTree::next_power_of_2(17), 32);
        assert_eq!(MerkleTree::next_power_of_2(2345), 4096);
        assert_eq!(MerkleTree::next_power_of_2(85191), 1024 * 128);
        assert_eq!(
            MerkleTree::next_power_of_2(102349872309),
            1024 * 1024 * 1024 * 128
        );
    }
    #[test]
    fn hash_tree_test() {
        let leafs: Vec<Sha256Hash> = vec![
            [0; 32], [1; 32], [2; 32], [3; 32], [4; 32], [5; 32], [6; 32], [7; 32],
        ];
        let nodes: Vec<Option<Sha256Hash>> = vec![None; 16];
        // Sanity check that concating actually produces the expected results...
        let hash_2_input = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1,
        ];
        assert_eq!(
            hash_bytes(&[[0; 32], [1; 32]].concat()),
            hash_bytes(&hash_2_input)
        );
        // This more repeated below, but let's be sure.
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 2),
            hash_bytes(&hash_2_input)
        );

        // values at the bottom of the tree should just be the transasction hashes in the leafs
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 1),
            [0; 32]
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 3),
            [1; 32]
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 5),
            [2; 32]
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 7),
            [3; 32]
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 9),
            [4; 32]
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 11),
            [5; 32]
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 13),
            [6; 32]
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 15),
            [7; 32]
        );

        let hash_2 = hash_bytes(&[[0; 32], [1; 32]].concat());
        let hash_6 = hash_bytes(&[[2; 32], [3; 32]].concat());
        let hash_10 = hash_bytes(&[[4; 32], [5; 32]].concat());
        let hash_14 = hash_bytes(&[[6; 32], [7; 32]].concat());
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 2),
            hash_2
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 6),
            hash_6
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 10),
            hash_10
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 14),
            hash_14
        );

        let hash_4 = hash_bytes(&[hash_2, hash_6].concat());
        let hash_12 = hash_bytes(&[hash_10, hash_14].concat());
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 4),
            hash_4
        );
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 12),
            hash_12
        );

        let hash_8 = hash_bytes(&[hash_4, hash_12].concat());
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 8),
            hash_8
        );
    }

    #[test]
    fn merkle_tree_test() {
        let timestamp_generator = make_timestamp_generator_for_test();
        let keypair = Keypair::new();
        let mut transactions = vec![];
        for _i in 0..16 {
            timestamp_generator.advance(1);
            transactions.push(Transaction::new(
                timestamp_generator.get_timestamp(),
                vec![],
                vec![],
                TxType::Seed,
                vec![],
                keypair.get_secret_key(),
            ));
        }

        let mut leafs = vec![];
        for tx in &transactions {
            leafs.push(*tx.get_hash());
        }

        timestamp_generator.advance(1);
        let mock_block_1: Box<dyn RawBlock> = Box::new(MockRawBlockForBlockchain::new(
            1,
            1000,
            [1; 32],
            [0; 32],
            timestamp_generator.get_timestamp(),
            transactions.clone(),
        ));

        let merkle_tree = MerkleTree::new(mock_block_1.transactions_iter());

        let nodes: Vec<Option<Sha256Hash>> = vec![None; 16];
        assert_eq!(
            MerkleTree::compute_node_hash_at_index(&nodes, &leafs, 8),
            merkle_tree.get_root().unwrap()
        );

        let hash_2 =
            hash_bytes(&[*transactions[0].get_hash(), *transactions[1].get_hash()].concat());
        let hash_6 =
            hash_bytes(&[*transactions[2].get_hash(), *transactions[3].get_hash()].concat());
        let hash_10 =
            hash_bytes(&[*transactions[4].get_hash(), *transactions[5].get_hash()].concat());
        let hash_14 =
            hash_bytes(&[*transactions[6].get_hash(), *transactions[7].get_hash()].concat());

        let hash_4 = hash_bytes(&[hash_2, hash_6].concat());
        let hash_12 = hash_bytes(&[hash_10, hash_14].concat());
        let hash_8 = hash_bytes(&[hash_4, hash_12].concat());

        assert_eq!(hash_8, merkle_tree.get_root().unwrap());

        let sub_tree_1 = merkle_tree.make_sub_tree_for_index(1);
        let sub_tree_9 = merkle_tree.make_sub_tree_for_index(9);

        assert_eq!(sub_tree_1.side_hashes[0], hash_12);
        assert_eq!(sub_tree_1.side_hashes[1], hash_6);
        assert_eq!(&sub_tree_1.side_hashes[2], transactions[1].get_hash());

        assert_eq!(sub_tree_9.side_hashes[0], hash_4);
        assert_eq!(sub_tree_9.side_hashes[1], hash_14);
        assert_eq!(&sub_tree_9.side_hashes[2], transactions[5].get_hash());
        assert!(merkle_tree.validate_sub_tree(sub_tree_1));
        assert!(merkle_tree.validate_sub_tree(sub_tree_9));
    }
}
