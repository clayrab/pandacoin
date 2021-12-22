use base58::FromBase58;
use blake3::{join::RayonJoin, Hasher};
use secp256k1::SecretKey;
pub use secp256k1::{Message, PublicKey, Signature, SECP256K1};
use sha2::{Digest, Sha256};
use std::convert::TryInto;

use crate::types::{Secp256k1SignatureCompact, Sha256Hash};
pub const PARALLEL_HASH_BYTE_THRESHOLD: usize = 128_000;

/// Hash the message string with sha256 for signing by secp256k1 and return as byte array
pub fn make_message_from_string(message_string: &str) -> Sha256Hash {
    let mut hasher = Sha256::new();
    hasher.update(message_string.as_bytes());
    let hashvalue = hasher.finalize();
    hashvalue.as_slice().try_into().unwrap()
}

pub fn make_message(message: &[u8]) -> Sha256Hash {
    let mut hasher = Sha256::new();
    hasher.update(message);
    let hashvalue = hasher.finalize();
    hashvalue.as_slice().try_into().unwrap()
}

pub fn hash_bytes(data: &[u8]) -> Sha256Hash {
    let mut hasher = Hasher::new();
    // Hashing in parallel can be faster if large enough
    // TODO: Blake3 has benchmarked 128 kb as the cutoff.
    if data.len() > PARALLEL_HASH_BYTE_THRESHOLD {
        hasher.update(data);
    } else {
        hasher.update_with_join::<RayonJoin>(data);
    }
    hasher.finalize().into()
}

pub fn sign_message(message_bytes: &[u8], secret_key: &SecretKey) -> Secp256k1SignatureCompact {
    let msg = Message::from_slice(&make_message(message_bytes)).unwrap();
    let sig = SECP256K1.sign(&msg, secret_key);
    sig.serialize_compact()
}

/// Verify a message signed by secp256k1. Message is a plain string. Sig and pubkey should be base58 encoded.
pub fn verify_string_message(message: &str, sig: &str, public_key: &str) -> bool {
    // TODO: Can we just use from_hashed_data?
    // see https://docs.rs/secp256k1/0.20.3/secp256k1/
    let message = Message::from_slice(&make_message_from_string(message)).unwrap();
    let sig = Signature::from_der(&String::from(sig).from_base58().unwrap()).unwrap();
    let public_key =
        PublicKey::from_slice(&String::from(public_key).from_base58().unwrap()).unwrap();
    SECP256K1.verify(&message, &sig, &public_key).is_ok()
}

/// Verify a message signed by secp256k1. Message is a byte array. Sig and pubkey should be base58 encoded.
pub fn verify_bytes_message(message_bytes: &[u8], sig_vec: &Vec<u8>, address: &Vec<u8>) -> bool {
    let sig = Signature::from_compact(&sig_vec[..]).unwrap();
    let public_key = PublicKey::from_slice(&address.clone()).unwrap();
    let message = Message::from_slice(&make_message(message_bytes)).unwrap();
    SECP256K1.verify(&message, &sig, &public_key).is_ok()
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use clap::Clap;

    use crate::{command_line_opts::CommandLineOpts, keypair_store::KeypairStore};

    use super::*;

    #[test]
    fn hash_test() {
        let vec: Vec<u8> = vec![0; 32];
        let hash = hash_bytes(&vec);
        assert_eq!(
            hash,
            [
                42, 218, 131, 193, 129, 154, 83, 114, 218, 225, 35, 143, 193, 222, 209, 35, 200,
                16, 79, 218, 161, 88, 98, 170, 238, 105, 66, 138, 24, 32, 252, 218
            ]
        );
    }

    #[test]
    fn make_message_from_string_test() {
        make_message_from_string("foobarbaz");
        make_message_from_string("1231231231");
        make_message_from_string("");
        assert!(true);
    }
    #[test]
    fn test_signatures() {
        let command_line_opts = Arc::new(CommandLineOpts::parse_from(&[
            "pandacoin",
            "--password",
            "asdf",
        ]));
        let keypair_store = KeypairStore::new_mock(command_line_opts);

        let msg = [0, 1, 2, 3];
        let signature = sign_message(&msg, keypair_store.get_keypair().get_secret_key());
        let is_valid = verify_bytes_message(
            &msg,
            &signature.to_vec(),
            &keypair_store
                .get_keypair()
                .get_public_key()
                .serialize()
                .to_vec(),
        );
        assert!(is_valid);
    }
}
