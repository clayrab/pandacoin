use clap::Clap;
// // cargo run -- --help
// // cargo run -- --key-path boom.txt

// //CommandLineOpts::parse();

//CommandLineOpts
lazy_static! {
    /// A global reference to the KeypairStore
    pub static ref COMMAND_LINE_OPTS_GLOBAL: CommandLineOpts = CommandLineOpts::parse();
}

/// This is the Clap options structure which stores all command line flags passed by the user to the application
// This should be moved to it's own module, but at the moment it is only used by KeypairStore.
#[derive(Clap, Debug)]
#[clap(name = "pandacoin")]
pub struct CommandLineOpts {
    /// Path to key-file
    #[clap(short, long, default_value = "./keyFile")]
    pub key_path: String,
    #[clap(short, long)]
    pub password: Option<String>,
    #[clap(short, long)]
    pub genesis: bool,
}