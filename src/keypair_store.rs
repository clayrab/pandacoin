/*!

# Keypair Store

This manages a keypair for the user through a CLI. It will encrypt, decrypt, and store a single keypair which the node
will use for signing and as the keypair for a wallet to receive rewards and to receive mini-blocks.

*/
extern crate rpassword;

use crate::command_line_opts::COMMAND_LINE_OPTS_GLOBAL;
use crate::crypto::hash_bytes;
use crate::keypair::Keypair;
use aes::Aes128;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use clap::Clap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::path::Path;

lazy_static! {
    /// A global reference to the KeypairStore
    pub static ref KEYPAIRSTORE_GLOBAL: KeypairStore = KeypairStore::new();
}

// create an alias for convenience
type Aes128Cbc = Cbc<Aes128, Pkcs7>;

// cargo run -- --help
// cargo run -- --key-path boom.txt

// This is the Clap options structure which stores all command line flags passed by the user to the application
// // This should be moved to it's own module, but at the moment it is only used by KeypairStore.
// #[derive(Clap, Debug)]
// #[clap(name = "bkc_rust")]
// struct CommandLineOpts {
//     /// Path to key-file
//     #[clap(short, long, default_value = "./keyFile")]
//     key_path: String,
//     #[clap(short, long)]
//     password: Option<String>,
// }

/// This manages a keypair for the user through a CLI. It will encrypt, decrypt, and store a single keypair which the node
/// will use for signing and as the keypair for a wallet to receive rewards and to receive mini-blocks.
#[derive(Debug, Clone)]
pub struct KeypairStore {
    /// The keypair
    keypair: Keypair,
}

impl KeypairStore {
    /// Create new `KeypairStore`.
    pub fn new() -> Self {
        //let opts = COMMAND_LINE_OPTS_GLOBAL;
        let path = Path::new(&COMMAND_LINE_OPTS_GLOBAL.key_path);
        let decrypted_buffer: Vec<u8>;

        if !path.exists() {
            println!("Creating key file");
            decrypted_buffer = KeypairStore::create_key_file(&COMMAND_LINE_OPTS_GLOBAL.key_path, &COMMAND_LINE_OPTS_GLOBAL.password);
        } else {
            println!("Reading key file");
            decrypted_buffer = KeypairStore::read_key_file(&COMMAND_LINE_OPTS_GLOBAL.key_path, &COMMAND_LINE_OPTS_GLOBAL.password);
        }
        let keypair = Keypair::from_secret_slice(&decrypted_buffer).unwrap();
        println!("{:?}", keypair.get_address());
        KeypairStore { keypair: keypair }
    }

    /// get the keypair
    pub fn get_keypair(&self) -> &Keypair {
        &self.keypair
    }

    fn create_key_file(key_file_path: &str, opts_password: &Option<String>) -> Vec<u8> {
        let password: String;
        if opts_password.is_none() {
            password = rpassword::prompt_password_stdout("Password: ").unwrap();
        } else {
            password = String::from(opts_password.as_deref().unwrap());
        }

        let (key, iv) = KeypairStore::create_primitives_from_password(&password);
        let cipher = Aes128Cbc::new_from_slices(&key, &iv).unwrap();

        let keypair = Keypair::new();
        let ciphertext = cipher.encrypt_vec(&keypair.get_secret_key()[..]);

        let mut file = File::create(key_file_path).unwrap();
        file.write_all(ciphertext.as_ref()).unwrap();

        keypair.get_secret_key()[..].to_vec()
    }

    fn read_key_file(key_file_path: &str, opts_password: &Option<String>) -> Vec<u8> {
        let password: String;
        if opts_password.is_none() {
            password = rpassword::prompt_password_stdout("Password: ").unwrap();
        } else {
            password = String::from(opts_password.as_deref().unwrap());
        }

        let mut file = File::open(key_file_path).unwrap();
        let mut encrypted_buffer = vec![];
        file.read_to_end(&mut encrypted_buffer).unwrap();

        KeypairStore::decrypt_key_file(encrypted_buffer, &password)
    }

    fn decrypt_key_file(ciphertext: Vec<u8>, password: &str) -> Vec<u8> {
        let (key, iv) = KeypairStore::create_primitives_from_password(password);
        let cipher = Aes128Cbc::new_from_slices(&key, &iv).unwrap();
        cipher.decrypt_vec(&ciphertext).unwrap()
    }

    fn create_primitives_from_password(password: &str) -> ([u8; 16], [u8; 16]) {
        let hash = hash_bytes(password.as_bytes());
        let mut key: [u8; 16] = [0; 16];
        let mut iv: [u8; 16] = [0; 16];
        key.clone_from_slice(&hash[0..16]);
        iv.clone_from_slice(&hash[16..32]);
        (key, iv)
    }
}
