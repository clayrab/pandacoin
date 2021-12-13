use std::sync::Arc;

use clap::Clap;

use crate::{
    command_line_opts::CommandLineOpts, keypair_store::KeypairStore,
    test_utilities::mock_timestamp_generator::MockTimestampGenerator,
    timestamp_generator::AbstractTimestampGenerator,
};

pub fn make_keypair_store_for_test() -> KeypairStore {
    let command_line_opts = Arc::new(CommandLineOpts::parse_from(&[
        "pandacoin",
        "--password",
        "asdf",
    ]));
    KeypairStore::new_mock(command_line_opts)
}

pub fn make_timestamp_generator_for_test() -> Arc<Box<dyn AbstractTimestampGenerator + Send + Sync>>
{
    Arc::new(Box::new(MockTimestampGenerator::new()))
}
