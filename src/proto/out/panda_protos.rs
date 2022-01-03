#[derive(Clone, Eq, Hash, PartialEq, ::prost::Message)]
pub struct OutputIdProto {
    #[prost(bytes = "vec", tag = "1")]
    pub tx_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub output_ordinal: u32,
}
#[derive(Clone, Eq, Hash, PartialEq, ::prost::Message)]
pub struct OutputProto {
    #[prost(bytes = "vec", tag = "1")]
    pub receiver: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub amount: u64,
}
#[derive(Clone, Eq, Hash, PartialEq, ::prost::Message)]
pub struct TransactionProto {
    #[prost(uint64, tag = "1")]
    pub timestamp: u64,
    #[prost(message, repeated, tag = "2")]
    pub inputs: ::prost::alloc::vec::Vec<OutputIdProto>,
    #[prost(message, repeated, tag = "3")]
    pub outputs: ::prost::alloc::vec::Vec<OutputProto>,
    #[prost(enumeration = "transaction_proto::TxType", tag = "4")]
    pub txtype: i32,
    #[prost(bytes = "vec", tag = "5")]
    pub message: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "7")]
    pub broker: ::prost::alloc::vec::Vec<u8>,
}
/// Nested message and enum types in `TransactionProto`.
pub mod transaction_proto {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum TxType {
        Normal = 0,
        Seed = 1,
        Service = 2,
    }
}
#[derive(Clone, Eq, Hash, PartialEq, ::prost::Message)]
pub struct MiniBlockProto {
    #[prost(bytes = "vec", tag = "1")]
    pub receiver: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub creator: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag = "4")]
    pub transactions: ::prost::alloc::vec::Vec<TransactionProto>,
}
#[derive(Clone, Eq, Hash, PartialEq, ::prost::Message)]
pub struct RawBlockProto {
    #[prost(uint32, tag = "1")]
    pub id: u32,
    #[prost(uint64, tag = "2")]
    pub timestamp: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub previous_block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub creator: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    pub merkle_root: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, repeated, tag = "8")]
    pub mini_blocks: ::prost::alloc::vec::Vec<MiniBlockProto>,
}
