//! Access to the checked-in WASM fixtures.
//!
//! See `README.md` in this directory for where each fixture came from.
//!
//! Each integration test binary compiles its own copy of this module, so not
//! every fixture is used in every one.
#![allow(dead_code)]

/// A contract built from `fixtures/testdata-contract/`, declaring one of every
/// kind of spec entry.
pub fn testdata() -> &'static [u8] {
    include_bytes!("testdata.wasm")
}

/// A real contract deployed to testnet and built with an older SDK.
pub fn oracle() -> &'static [u8] {
    include_bytes!("oracle.wasm")
}
