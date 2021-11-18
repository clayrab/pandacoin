use clap::Clap;

/// This is the Clap options structure which stores all command line flags passed by the user to the application
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

    #[clap(short, long)]
    pub nocapture: bool,
}
#[cfg(test)]
mod test {

    // use super::*;
    // #[test]
    // fn test_complete_name() {
    //     assert_eq!(
    //         Some(String::from("asdf")),
    //         COMMAND_LINE_OPTS_GLOBAL.get().unwrap().password
    //     );
    // }
}
