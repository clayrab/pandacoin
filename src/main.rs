use clap::Clap;
use log::{debug, error, info, warn};
use pandacoin::block::PandaBlock;
use pandacoin::keypair_store::KeypairStore;
use pandacoin::timestamp_generator::{AbstractTimestampGenerator, SystemTimestampGenerator};
use std::env;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tracing::{event, Level};
use tracing_subscriber;

use pandacoin::blockchain::{AbstractBlockchain, AddBlockEvent, Blockchain};
use pandacoin::command_line_opts::CommandLineOpts;
use pandacoin::utxoset::{AbstractUtxoSet, UtxoSet};

#[tokio::main]
pub async fn main() -> pandacoin::Result<()> {
    // Initialize all "globals", timestamp generator, command line opts, keypair store, utxoset, blockchain, etc
    let timestamp_generator = Box::new(SystemTimestampGenerator::new());
    let command_line_opts = Arc::new(CommandLineOpts::parse());
    let keypair_store = KeypairStore::new(command_line_opts.clone());
    let utxoset_ref: Arc<RwLock<Box<dyn AbstractUtxoSet + Send + Sync>>> =
        Arc::new(RwLock::new(Box::new(UtxoSet::new())));
    let blockchain_mutex_ref =
        Arc::new(RwLock::new(Box::new(Blockchain::new(utxoset_ref.clone()))));

    println!("WELCOME TO PANDACOIN!");
    if env::var("RUST_LOG").is_err() {
        println!("Setting Log Level to INFO. Use RUST_LOG=[level] to set. Accepts trace, info, debug, warn, error.");
        env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    event!(Level::INFO, "Node Started");

    debug!("this is a debug {}", "message");
    warn!("this is a warning");
    info!("this is info {}", timestamp_generator.get_timestamp());
    error!("this is printed by default");

    println!("Key: {}", keypair_store.get_keypair().get_public_key());
    if command_line_opts.genesis {
        println!("***********************************************");
        println!("******************** GENESIS ******************");
        println!("***********************************************");
        println!("*********** CREATING GENESIS BLOCK ************");
        println!("***********************************************");
        let mut blockchain = blockchain_mutex_ref.write().await;
        let genesis_block = PandaBlock::new_genesis_block(
            keypair_store.get_keypair().get_public_key().clone(),
            timestamp_generator.get_timestamp(),
        );
        let result = blockchain.add_block(Box::new(genesis_block)).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
    }

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
