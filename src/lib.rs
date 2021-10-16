pub mod block;
pub mod block_fee_manager;
pub mod blockchain;
pub mod command_line_opts;
pub mod constants;
pub mod crypto;
pub mod forktree;
pub mod keypair;
pub mod keypair_store;
pub mod longest_chain_queue;
pub mod output;
#[path = "proto/out/panda_protos.rs"]
pub mod panda_protos;
pub mod timestamp_generator;
pub mod transaction;
pub mod types;

pub mod test_utilities;

// pub mod src/proto/out/p;
//
// Not sure what the purpose of this was, I suspect that if we didn't need to use Eq on the proto, this would be useful.
//
// pub mod slip_proto_direct {
//     include!(concat!(env!("OUT_DIR"), "/slip_proto.rs"));
// }

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate tracing;

/// Error returned by most functions.
///
/// When writing a real application, one might want to consider a specialized
/// error handling crate or defining an error type as an `enum` of causes.
/// However, most time using a boxed `std::error::Error` is sufficient.
///
/// For performance reasons, boxing is avoided in any hot path. For example, in
/// `parse`, a custom error `enum` is defined. This is because the error is hit
/// and handled during normal execution when a partial frame is received on a
/// socket. `std::error::Error` is implemented for `parse::Error` which allows
/// it to be converted to `Box<dyn std::error::Error>`.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// A specialized `Result` type for operations.
///
/// This is defined as a convenience.
pub type Result<T> = std::result::Result<T, Error>;
