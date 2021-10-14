use std::env;
use clap::{App, Arg};
use pandacoin::panda_protos::RawBlockProto;
use pandacoin::timestamp_generator::TIMESTAMP_GENERATOR_GLOBAL;
use tokio::signal;
use tracing::{Level, event};
use tracing_subscriber;
use log::{info, warn, debug, error};

use pandacoin::command_line_opts::COMMAND_LINE_OPTS_GLOBAL;
use pandacoin::blockchain::{AddBlockEvent, BLOCKCHAIN_GLOBAL};

#[tokio::main]
pub async fn main() -> pandacoin::Result<()> {
    println!("WELCOME TO PANDA COIN!");
    if env::var("RUST_LOG").is_err() {
        println!("Setting Log Level to INFO. Use RUST_LOG=[level] to set. Accepts trace, info, debug, warn, error.");
        env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    event!(
        Level::INFO,
        "Node Started"
    );

    debug!("this is a debug {}", "message");
    warn!("this is a warniig");
    info!("this is info {}", TIMESTAMP_GENERATOR_GLOBAL.get_timestamp());
    error!("this is printed by default");
    
    if COMMAND_LINE_OPTS_GLOBAL.genesis {
        println!("***********************************************");
        println!("******************** GENESIS ******************");
        println!("***********************************************");
        println!("*********** CREATING GENESIS BLOCK ************");
        println!("***********************************************");
        let blockchain_arc = BLOCKCHAIN_GLOBAL.clone();
        let mut blockchain = blockchain_arc.write().await;
        let genesis_block = RawBlockProto::create_genesis_block();
        let result = blockchain.add_block(genesis_block).await;
        assert_eq!(result, AddBlockEvent::AcceptedAsLongestChain);
    }
    tokio::select! {
        // res = consensus.run() => {
        //     if let Err(err) = res {
        //         eprintln!("{:?}", err);
        //     }
        // },
        _ = signal::ctrl_c() => {
            println!("Shutting down!")
        }
    }
    Ok(())
}
