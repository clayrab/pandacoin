use crate::{
    panda_protos::{OutputIdProto, OutputProto},
    types::{PandaAddress, Sha256Hash},
};
use secp256k1::PublicKey;
use std::{
    convert::TryInto,
    hash::{Hash, Hasher},
};

/// A record of owernship of funds on the network
impl Hash for OutputIdProto {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.slip_ordinal.to_ne_bytes());
        state.write(&self.tx_id)
    }
}

impl OutputIdProto {
    /// Create new `OutputIdProto`
    pub fn new(tx_id: Sha256Hash, slip_ordinal: u32) -> Self {
        OutputIdProto {
            tx_id: tx_id.to_vec(),
            slip_ordinal: slip_ordinal,
        }
    }

    /// Returns the `Transaction` id the slip originated from
    pub fn tx_id(&self) -> Sha256Hash {
        self.tx_id.clone().try_into().unwrap()
    }

    /// Returns the `Slip`
    pub fn slip_ordinal(&self) -> u32 {
        self.slip_ordinal
    }
}

impl OutputProto {
    pub fn new(address: PublicKey, amount: u64) -> OutputProto {
        OutputProto {
            receiver: address.serialize().to_vec(),
            amount: amount,
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
