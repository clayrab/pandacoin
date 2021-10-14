use std::{env, fs::File, io::Write, path::Path};
extern crate prost_build;
//
// To build protos:
//   cargo run --bin build_protos --features="build_deps"
//
// After building, we manually add the "Eq" to the derive macros for OutputProto and OutputIdProto.
// This is not done automatically by prost because it is possible for a proto to include floating
// point types, however, in our case there is no problem with deriving Eq and it is needed to be able
// to hash these types.
fn main() {
    let mut prost_build = prost_build::Config::new();
    std::env::set_var("OUT_DIR", "./src/proto/out");
    // Enable a protoc experimental feature.
    prost_build.protoc_arg("--experimental_allow_proto3_optional");
    prost_build::compile_protos(&["src/proto/protos.proto"], &["src/"]).unwrap();
    println!("SLIPS HAVE BEEN COMPILED TO ./src/proto/out");
    println!("OUTPUT AND OUTPUTID PROTOS MUST HAVE 'Eq' ADDED TO THEIR derive() MACROS!");
}
