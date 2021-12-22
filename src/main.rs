use clap::Clap;
use log::{debug, error, info, warn};
use pandacoin::block::PandaBlock;
use pandacoin::block_fee_manager::BlockFeeManager;
use pandacoin::blocks_database::BlocksDatabase;
use pandacoin::constants::Constants;
use pandacoin::fork_manager::ForkManager;
use pandacoin::keypair_store::KeypairStore;
use pandacoin::longest_chain_queue::LongestChainQueue;
use pandacoin::mempool::{AbstractMempool, Mempool};
use pandacoin::miniblock_manager::MiniblockManager;
use pandacoin::shutdown_signals::signal_for_shutdown;
use pandacoin::timestamp_generator::{AbstractTimestampGenerator, SystemTimestampGenerator};
use std::env;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{event, Level};

use pandacoin::blockchain::Blockchain;
use pandacoin::command_line_opts::CommandLineOpts;
use pandacoin::utxoset::{AbstractUtxoSet, UtxoSet};

#[tokio::main]
pub async fn main() -> pandacoin::Result<()> {
    tracing_subscriber::fmt::init();

    // Initialize all "globals", timestamp generator, command line opts, keypair store, utxoset, blockchain, etc
    let (shutdown_channel_sender, shutdown_channel_receiver) = broadcast::channel(1);
    let (shutdown_waiting_sender, mut shutdown_waiting_receiver) = mpsc::channel::<()>(1);
    let constants = Arc::new(Constants::new());
    let timestamp_generator: Arc<Box<dyn AbstractTimestampGenerator>> =
        Arc::new(Box::new(SystemTimestampGenerator::new()));
    let command_line_opts = Arc::new(CommandLineOpts::parse());
    let keypair_store = Arc::new(KeypairStore::new(command_line_opts.clone()));

    let genesis_block;
    if command_line_opts.genesis {
        println!("***********************************************");
        println!("******************** GENESIS ******************");
        println!("***********************************************");
        println!("*********** CREATING GENESIS BLOCK ************");
        println!("***********************************************");
        genesis_block = PandaBlock::new_genesis_block(
            *keypair_store.get_keypair().get_public_key(),
            timestamp_generator.get_timestamp(),
            constants.get_starting_block_fee(),
        );
    } else {
        // TODO read the genesis block from disk
        genesis_block = PandaBlock::new_genesis_block(
            *keypair_store.get_keypair().get_public_key(),
            timestamp_generator.get_timestamp(),
            constants.get_starting_block_fee(),
        );
    }

    let longest_chain_queue = LongestChainQueue::new(&genesis_block);
    let fork_manager = ForkManager::new(&genesis_block, constants.clone());
    let block_fee_manager = BlockFeeManager::new(constants.clone());
    let blocks_database = BlocksDatabase::new(genesis_block);
    let utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
        Arc::new(RwLock::new(Box::new(UtxoSet::new(constants.clone()))));

    let mempool_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>> =
        Arc::new(RwLock::new(Box::new(
            Mempool::new(
                utxoset_ref.clone(),
                shutdown_channel_receiver,
                shutdown_waiting_sender.clone(),
            )
            .await,
        )));
    let _blockchain_mutex_ref = Arc::new(RwLock::new(Box::new(
        Blockchain::new(
            fork_manager,
            longest_chain_queue,
            blocks_database,
            block_fee_manager,
            constants.clone(),
            utxoset_ref.clone(),
            mempool_ref.clone(),
        )
        .await,
    )));
    let _miniblock_manager_mutex_ref = Arc::new(RwLock::new(Box::new(MiniblockManager::new(
        utxoset_ref.clone(),
        mempool_ref.clone(),
        timestamp_generator.clone(),
        keypair_store.clone(),
    ))));

    println!("WELCOME TO PANDACOIN!");
    if env::var("RUST_LOG").is_err() {
        println!("Setting Log Level to INFO. Use RUST_LOG=[level] to set. Accepts trace, info, debug, warn, error.");
        env::set_var("RUST_LOG", "info");
    }

    event!(Level::INFO, "Node Started");

    debug!("this is a debug {}", "message");
    warn!("this is a warning");
    info!("this is info {}", timestamp_generator.get_timestamp());
    error!("this is printed by default");

    println!("Key: {}", keypair_store.get_keypair().get_public_key());

    // Run whatever tasks compose the application
    tokio::spawn(async move { run().await });

    // Wait for a shutdown signal
    match signal_for_shutdown().await {
        Ok(()) => {
            println!("Shutting down");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }

    // The shutdown signal has been received, send a shutdown message to each async task
    let _res = shutdown_channel_sender.send(());

    // Wait for everyone to shutdown gracefully! We do this by dropping the reference
    // to the waiting_sender, the recv() will then throw an error when the last reference
    // to the sender is dropped. Each async task should hold a reference to the sender so that
    // this works properly. See https://tokio.rs/tokio/topics/shutdown for details.
    drop(shutdown_waiting_sender);
    let _ = shutdown_waiting_receiver.recv().await;

    println!("Shutdown Complete");
    Ok(())
}

async fn run() -> pandacoin::Result<()> {
    loop {
        //println!("loop");
    }
    // Ok(())
}
