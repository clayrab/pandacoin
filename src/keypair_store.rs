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
use once_cell::sync::OnceCell;
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::path::Path;

pub static KEYPAIRSTORE_GLOBAL: OnceCell<KeypairStore> = OnceCell::new();

// create an alias for convenience
type Aes128Cbc = Cbc<Aes128, Pkcs7>;

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
        let path = Path::new(&COMMAND_LINE_OPTS_GLOBAL.get().unwrap().key_path);
        let decrypted_buffer: Vec<u8>;
        if !path.exists() {
            println!("Creating key file");
            decrypted_buffer = KeypairStore::create_key_file(
                &COMMAND_LINE_OPTS_GLOBAL.get().unwrap().key_path,
                &COMMAND_LINE_OPTS_GLOBAL.get().unwrap().password,
            );
        } else {
            println!("Reading key file");
            decrypted_buffer = KeypairStore::read_key_file(
                &COMMAND_LINE_OPTS_GLOBAL.get().unwrap().key_path,
                &COMMAND_LINE_OPTS_GLOBAL.get().unwrap().password,
            );
        }
        let keypair = Keypair::from_secret_slice(&decrypted_buffer).unwrap();
        KeypairStore { keypair: keypair }
    }
    /// Create new `KeypairStore` for testing from existing wallet in test data.
    pub fn new_mock() -> Self {
        let decrypted_buffer: Vec<u8>;
        decrypted_buffer =
            KeypairStore::read_key_file(&"data/test/testwallet", &Some(String::from("asdf")));
        let keypair = Keypair::from_secret_slice(&decrypted_buffer).unwrap();
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

#[cfg(test)]
mod test {
    use crate::test_utilities::init_globals_for_tests;

    use super::*;

    #[tokio::test]
    async fn mock_keypair_store_test() {
        init_globals_for_tests();
        assert_eq!(
            "02fc238df3474c274887f85b3f36d3adffa5465f15840779da6fc82f912c4d1009",
            hex::encode(
                KEYPAIRSTORE_GLOBAL
                    .get()
                    .unwrap()
                    .get_keypair()
                    .get_public_key()
                    .serialize()
            )
        );
    }
}
