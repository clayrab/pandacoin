//
// This file was copied from the build directory and added to the source code.
// This is a workaround to be able to add derive(Eq), to the protos so they can
// be hashed by Hashmap. This is not normally permitted because any proto with
// a float cannot derive Eq.
//
// To regenerate this file:
//   cargo run --bin force-build --features="build_deps"
// The output can then be found in a file like this:
//  ./target/debug/build/pandacoin-276b819e77ee081e/out/slip_proto.rs
//
// TODO find a better way to resolve the issue described above. See also slip_proto_direct in lib.rs.
// 

/// This is an input, we refer to only outputs to avoid confusion, an input is simply a reference 
/// to an output which has been created in a tranasction which is already in a block.
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct OutputId {
    #[prost(bytes="vec", tag="1")]
    pub tx_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag="2")]
    pub slip_ordinal: u32,
}
#[derive(Clone, Eq, PartialEq, ::prost::Message)]
pub struct Output {
    #[prost(bytes="vec", tag="1")]
    pub receiver: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag="2")]
    pub amount: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionProto {
    #[prost(uint64, tag="1")]
    pub timestamp: u64,
    #[prost(message, repeated, tag="2")]
    pub inputs: ::prost::alloc::vec::Vec<OutputId>,
    #[prost(message, repeated, tag="3")]
    pub outputs: ::prost::alloc::vec::Vec<Output>,
    #[prost(enumeration="transaction_proto::TxType", tag="4")]
    pub r#txtype: i32,
    #[prost(bytes="vec", tag="5")]
    pub message: ::prost::alloc::vec::Vec<u8>,
}
/// Nested message and enum types in `TransactionProto`.
pub mod transaction_proto {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum TxType {
        Normal = 0,
        Service = 1,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockProto {
    #[prost(uint64, tag="1")]
    pub id: u64,
    #[prost(uint64, tag="2")]
    pub timestamp: u64,
    #[prost(bytes="vec", tag="3")]
    pub previous_block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="4")]
    pub creator: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="5")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag="6")]
    pub treasury: u64,
    #[prost(uint64, tag="7")]
    pub start_burnfee: u64,
}
