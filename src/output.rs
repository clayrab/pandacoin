use crate::{
    panda_protos::{OutputIdProto, OutputProto},
    types::{PandaAddress, Sha256Hash},
};
use secp256k1::PublicKey;
use std::convert::TryInto;

impl OutputIdProto {
    /// Create new `OutputIdProto`
    pub fn new(tx_id: Sha256Hash, output_ordinal: u32) -> Self {
        OutputIdProto {
            tx_id: tx_id.to_vec(),
            output_ordinal,
        }
    }

    /// Returns the `Transaction` id the output originated from
    pub fn tx_id(&self) -> Sha256Hash {
        self.tx_id.clone().try_into().unwrap()
    }

    /// Returns the `Slip`
    pub fn output_ordinal(&self) -> u32 {
        self.output_ordinal
    }
}

impl OutputProto {
    pub fn new(address: PublicKey, amount: u64) -> OutputProto {
        OutputProto {
            receiver: address.serialize().to_vec(),
            amount,
        }
    }

    /// Returns address in `Slip`
    pub fn address(&self) -> &PandaAddress {
        &self.receiver
    }

    /// Returns amount of COIN in `Slip`
    pub fn amount(&self) -> u64 {
        self.amount
    }
}
