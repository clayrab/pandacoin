use clap::Clap;
use log::{debug, error, info, warn};
use pandacoin::block::PandaBlock;
use pandacoin::constants::Constants;
use pandacoin::keypair_store::KeypairStore;
use pandacoin::mempool::{Mempool, AbstractMempool};
use pandacoin::miniblock_manager::MiniblockManager;
use pandacoin::timestamp_generator::{AbstractTimestampGenerator, SystemTimestampGenerator};
use std::env;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tracing::{event, Level};
use tracing_subscriber;

use pandacoin::blockchain::Blockchain;
use pandacoin::command_line_opts::CommandLineOpts;
use pandacoin::utxoset::{AbstractUtxoSet, UtxoSet};

#[tokio::main]
pub async fn main() -> pandacoin::Result<()> {
    tracing_subscriber::fmt::init();

    // Initialize all "globals", timestamp generator, command line opts, keypair store, utxoset, blockchain, etc
    let constants = Arc::new(Constants::new());
    let timestamp_generator = Box::new(SystemTimestampGenerator::new());
    let command_line_opts = Arc::new(CommandLineOpts::parse());
    let keypair_store = KeypairStore::new(command_line_opts.clone());
    
    let genesis_block;
    if command_line_opts.genesis {
        println!("***********************************************");
        println!("******************** GENESIS ******************");
        println!("***********************************************");
        println!("*********** CREATING GENESIS BLOCK ************");
        println!("***********************************************");
        genesis_block = PandaBlock::new_genesis_block(
            keypair_store.get_keypair().get_public_key().clone(),
            timestamp_generator.get_timestamp(),
            constants.get_starting_block_fee(),
        );
    } else {
        // TODO read the genesis block from disk
        genesis_block = PandaBlock::new_genesis_block(
            keypair_store.get_keypair().get_public_key().clone(),
            timestamp_generator.get_timestamp(),
            constants.get_starting_block_fee(),
        );
    }

    let utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
        Arc::new(RwLock::new(Box::new(UtxoSet::new(constants.clone()))));
    let _blockchain_mutex_ref =
        Arc::new(RwLock::new(Box::new(Blockchain::new(genesis_block, utxoset_ref.clone()).await)));
    let mempool_mutex_ref: Arc<RwLock<Box<dyn AbstractMempool + Send + Sync>>> =
        Arc::new(RwLock::new(Box::new(Mempool::new(utxoset_ref.clone()))));
    let _miniblock_manager_mutex_ref =
        Arc::new(RwLock::new(Box::new(MiniblockManager::new(utxoset_ref.clone(), mempool_mutex_ref.clone()))));
    
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

    tokio::select! {
        res = run() => {
            if let Err(err) = res {
                eprintln!("Runtime has thrown an error: {:?}", err);
            }
        },
        _ = signal::ctrl_c() => {
            println!("Shutting down!")
        }
    }
    Ok(())
}

async fn run() -> pandacoin::Result<()> {
    loop {}
    // Ok(())
}
