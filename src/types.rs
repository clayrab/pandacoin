/// The proto library will not decode to an array, only a vector.
/// This "type" only exists solely for the sake of semantics, so that it's clear what
/// sort of data the Vec<u8> is carrying
pub type PandaAddress = Vec<u8>;
/// Sha256Hash byte array type
pub type Sha256Hash = [u8; 32];
pub type Secp256k1SignatureCompact = [u8; 64];
