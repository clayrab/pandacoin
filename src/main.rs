use std::env;
use pandacoin::timestamp_generator::TIMESTAMP_GENERATOR_GLOBAL;
use tokio::signal;
use tracing::{Level, event};
use tracing_subscriber;
use log::{info, warn, debug, error};

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
