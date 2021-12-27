use crate::block::RawBlock;
use crate::panda_protos::OutputIdProto;
use crate::transaction::Transaction;
use crate::types::Sha256Hash;
use crate::utxoset::AbstractUtxoSet;
use crate::Error;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::sleep;

#[async_trait]
pub trait AbstractMempool: Debug {
    fn get_latest_block_id(&self) -> u32;
    async fn add_transaction(&mut self, transaction: Transaction) -> bool;
    fn roll_forward(&mut self, block: &Box<dyn RawBlock>);
    fn roll_back(&mut self, block: &Box<dyn RawBlock>);
    fn roll_forward_max_reorg(&mut self, block: &Box<dyn RawBlock>);
    fn get_current_set(&self) -> &HashSet<Sha256Hash>;
}

#[derive(Debug)]
struct MempoolContext {
    utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
}

#[derive(Debug)]
pub struct Mempool {
    // track the latest block in mempool itself so we don't have to get it from some Arc<Mutex> elsewhere
    block_count: u32,
    // The current set of transactions that we are the broker for
    current_set: HashSet<Sha256Hash>,
    // tx-hash -> tx
    transactions: HashMap<Sha256Hash, Transaction>,
    // maps inputs to transactions
    known_inputs: HashMap<OutputIdProto, HashSet<Sha256Hash>>,
    // Mempool Context
    context: MempoolContext,
}

impl Mempool {
    pub async fn run(_shutdown_waiting_sender: mpsc::Sender<()>) -> Result<(), Error> {
        loop {
            println!("tick");
            sleep(Duration::from_millis(1000)).await;
        }
    }
    // Mempool Constructor
    pub async fn new(
        utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>>,
        mut shutdown_channel_receiver: broadcast::Receiver<()>,
        shutdown_waiting_sender: mpsc::Sender<()>,
    ) -> Self {
        tokio::spawn(async move {
            let result = tokio::select! {
                // Do not clone this sender, we are using it for reference counting during graceful shutdown
                res = Mempool::run(shutdown_waiting_sender) => {
                    res
                },
                _ = shutdown_channel_receiver.recv() => {
                    println!("Received shutdown signal in Mempool");
                    Ok(())
                },
            };
            println!("Mempool result: {:?}", result);
        });

        Mempool {
            context: MempoolContext { utxoset_ref },
            block_count: 0,
            current_set: HashSet::new(),
            transactions: HashMap::new(),
            known_inputs: HashMap::new(),
        }
    }
}

#[async_trait]
impl AbstractMempool for Mempool {
    /// We track the latest block id internally in Mempool so we don't need to get it from longest chain
    /// or somewhere else via an Arc<Mutex<_>>
    fn get_latest_block_id(&self) -> u32 {
        if self.block_count >= 1 {
            self.block_count - 1
        } else {
            0
        }
    }
    /// get the current sent of transactions that could be made into a valid block
    fn get_current_set(&self) -> &HashSet<Sha256Hash> {
        &self.current_set
    }
    ///
    async fn add_transaction(&mut self, transaction: Transaction) -> bool {
        let utxoset = self.context.utxoset_ref.read().await;
        let mut is_spendable = true;
        for input in transaction.get_inputs() {
            is_spendable = is_spendable
                || utxoset
                    .is_output_spendable_at_block_id(input, self.get_latest_block_id())
                    .await;
        }
        let is_single_spender = utxoset
            .get_receiver_for_inputs(transaction.get_inputs())
            .is_some();
        if is_spendable && is_single_spender {
            let mut is_already_known = false;
            // for each input, track the tx hash
            for input in transaction.get_inputs() {
                self.known_inputs
                    .entry(input.clone())
                    .and_modify(|tx_set| {
                        // if any input is already recorded in known_inputs, this tx is already_known
                        is_already_known = true;
                        tx_set.insert(*transaction.get_hash());
                    })
                    .or_insert(HashSet::from([*transaction.get_hash()]));
            }
            if !is_already_known {
                self.current_set.insert(*transaction.get_hash());
            } else {
                // TODO there is a UX issue here. It might make more sense to allow a replace-by-fee style logic here, but first-seen
                // is I think a better UX and also much easier to implement.
                // To implement replace-by-fee, we must loop thought all the inputs used in the transaction and then get all the transactions
                // which are in conflict with the, sort by fee, and then apply them in order. For now we will do nothing and have a
                // first-seen policy.
            }
            self.transactions
                .insert(*transaction.get_hash(), transaction);
        }
        is_spendable
    }

    fn roll_forward(&mut self, block: &Box<dyn RawBlock>) {
        self.block_count += 1;
        // For each transaction in the block, remove all transactions from the mempool which was spending any of
        // their inputs(which should include the transaction itself)
        for transaction in block.transactions_iter() {
            for input in transaction.get_inputs() {
                if let Some(tx_set) = self.known_inputs.get(input) {
                    for tx_hash in tx_set {
                        self.current_set.remove(tx_hash);
                    }
                }
            }
        }
    }

    fn roll_back(&mut self, block: &Box<dyn RawBlock>) {
        self.block_count -= 1;
        for transaction in block.transactions_iter() {
            // if we had this transaction in mempool earlier, we can reuse it
            if let Some(_) = self.transactions.get(transaction.get_hash()) {
                self.current_set.insert(*transaction.get_hash());
            }
            for input in transaction.get_inputs() {
                self.known_inputs
                    .entry(input.clone())
                    .and_modify(|tx_set| {
                        tx_set.insert(*transaction.get_hash());
                    })
                    .or_insert(HashSet::from([*transaction.get_hash()]));
            }
        }
    }
    fn roll_forward_max_reorg(&mut self, block: &Box<dyn RawBlock>) {
        for transaction in block.transactions_iter() {
            for input in transaction.get_inputs() {
                self.known_inputs.remove(input);
            }
            self.transactions.remove(transaction.get_hash());
        }
    }
}

#[cfg(test)]
mod test {
    use super::{AbstractMempool, Mempool};
    use crate::{
        block::RawBlock,
        keypair::Keypair,
        panda_protos::{transaction_proto::TxType, OutputIdProto, OutputProto},
        test_utilities::{
            globals_init::make_timestamp_generator_for_test, mock_block::MockRawBlockForUTXOSet,
            mock_utxoset::MockUtxoSet,
        },
        transaction::Transaction,
        utxoset::AbstractUtxoSet,
    };
    use std::sync::Arc;
    use tokio::sync::{broadcast, mpsc, RwLock};

    #[tokio::test]
    async fn mempool_test() {
        let timestamp_generator = make_timestamp_generator_for_test();
        let keypair = Keypair::new();
        let input_a = OutputIdProto::new([1; 32], 0);
        let output_a_1 = OutputProto::new(*keypair.get_public_key(), 1);
        let output_a_2 = OutputProto::new(*keypair.get_public_key(), 2);

        let tx_a_1 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![input_a.clone()],
            vec![output_a_1.clone()],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );
        let tx_a_1_hash = *tx_a_1.get_hash();

        let tx_a_2 = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![input_a.clone()],
            vec![output_a_2],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );
        let tx_a_2_hash = *tx_a_2.get_hash();

        let input_b = OutputIdProto::new([2; 32], 0);
        let output_b = OutputProto::new(*keypair.get_public_key(), 1);
        let tx_b = Transaction::new(
            timestamp_generator.get_timestamp(),
            vec![input_b.clone()],
            vec![output_b.clone()],
            TxType::Normal,
            vec![],
            keypair.get_secret_key(),
        );
        let tx_b_hash = *tx_b.get_hash();

        let mock_block_a_1: Box<dyn RawBlock> = Box::new(MockRawBlockForUTXOSet::new(
            0,
            [1; 32],
            vec![tx_a_1.clone()],
        ));
        let mock_block_a_2: Box<dyn RawBlock> = Box::new(MockRawBlockForUTXOSet::new(
            0,
            [2; 32],
            vec![tx_a_1.clone()],
        ));
        let mock_block_b: Box<dyn RawBlock> =
            Box::new(MockRawBlockForUTXOSet::new(0, [3; 32], vec![tx_b.clone()]));

        let mut utxoset = MockUtxoSet::new();
        utxoset.insert_mock_output(output_a_1, input_a);
        let mock_utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
            Arc::new(RwLock::new(Box::new(utxoset)));

        let (_, shutdown_channel_receiver) = broadcast::channel(1);
        let (shutdown_waiting_sender, _) = mpsc::channel::<()>(1);
        let mut mempool = Mempool::new(
            mock_utxoset_ref.clone(),
            shutdown_channel_receiver,
            shutdown_waiting_sender.clone(),
        )
        .await;

        let added_a_1 = mempool.add_transaction(tx_a_1.clone()).await;
        let added_a_2 = mempool.add_transaction(tx_a_2.clone()).await;
        assert!(added_a_1);
        assert!(added_a_2);

        assert_eq!(mempool.get_current_set().len(), 1);
        assert!(mempool.get_current_set().get(&tx_a_1_hash).is_some());
        assert!(mempool.get_current_set().get(&tx_a_2_hash).is_none());
        assert_eq!(mempool.get_latest_block_id(), 0);

        mempool.roll_forward(&mock_block_a_1);

        assert_eq!(mempool.get_current_set().len(), 0);
        assert!(mempool.get_current_set().get(&tx_a_1_hash).is_none());
        assert!(mempool.get_current_set().get(&tx_a_2_hash).is_none());
        assert_eq!(mempool.get_latest_block_id(), 0);

        mempool.roll_back(&mock_block_a_1);

        assert_eq!(mempool.get_current_set().len(), 1);
        assert!(mempool.get_current_set().get(&tx_a_1_hash).is_some());
        assert!(mempool.get_current_set().get(&tx_a_2_hash).is_none());
        assert_eq!(mempool.get_latest_block_id(), 0);

        mempool.roll_forward(&mock_block_a_1);

        assert_eq!(mempool.get_current_set().len(), 0);
        assert!(mempool.get_current_set().get(&tx_a_1_hash).is_none());
        assert!(mempool.get_current_set().get(&tx_a_2_hash).is_none());
        assert_eq!(mempool.get_latest_block_id(), 0);

        mempool.roll_forward(&mock_block_a_2);
        assert_eq!(mempool.get_latest_block_id(), 1);

        let added_b = mempool.add_transaction(tx_b.clone()).await;
        assert!(added_b);

        assert_eq!(mempool.get_current_set().len(), 1);
        assert!(mempool.get_current_set().get(&tx_a_1_hash).is_none());
        assert!(mempool.get_current_set().get(&tx_a_2_hash).is_none());
        assert!(mempool.get_current_set().get(&tx_b_hash).is_some());
        assert_eq!(mempool.get_latest_block_id(), 1);

        mempool.roll_forward(&mock_block_b);

        assert_eq!(mempool.get_current_set().len(), 0);
        assert!(mempool.get_current_set().get(&tx_a_1_hash).is_none());
        assert!(mempool.get_current_set().get(&tx_a_2_hash).is_none());
        assert!(mempool.get_current_set().get(&tx_b_hash).is_none());
        assert_eq!(mempool.get_latest_block_id(), 2);
    }
}
