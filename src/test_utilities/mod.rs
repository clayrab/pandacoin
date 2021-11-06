use std::sync::Arc;

use clap::Clap;
use tokio::sync::RwLock;

use crate::{
    blockchain::BLOCKCHAIN_GLOBAL,
    command_line_opts::{CommandLineOpts, COMMAND_LINE_OPTS_GLOBAL},
    keypair_store::{KeypairStore, KEYPAIRSTORE_GLOBAL},
    timestamp_generator::TIMESTAMP_GENERATOR_GLOBAL,
};

use self::{mock_blockchain::MockBlockchain, mock_timestamp_generator::MockTimestampGenerator};

pub mod mock_block;
pub mod mock_blockchain;
pub mod mock_timestamp_generator;

pub fn init_globals_for_tests() {
    KEYPAIRSTORE_GLOBAL
        .set(KeypairStore::new_mock())
        .unwrap_or(());
    COMMAND_LINE_OPTS_GLOBAL
        .set(CommandLineOpts::parse_from(&[
            "pandacoin",
            "--password",
            "asdf",
        ]))
        .unwrap_or(());
    TIMESTAMP_GENERATOR_GLOBAL
        .set(Box::new(MockTimestampGenerator::new()))
        .unwrap_or(());
    BLOCKCHAIN_GLOBAL
        .set(Arc::new(RwLock::new(Box::new(MockBlockchain::new()))))
        .unwrap_or(());
}
