//! A contract that exists purely to be inspected.
//!
//! It declares at least one of every `SCSpecEntry` kind and exercises the
//! interesting corners of the type grammar (nested generics, tuples, `BytesN`,
//! results, options), so Lumenscan's decoder is tested against real,
//! SDK-produced XDR rather than hand-written bytes.
//!
//! Build with `make fixtures` from the repository root.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contractmeta, contracttype, Address,
    Bytes, BytesN, Duration, Env, Map, String, Symbol, Timepoint, Vec,
};

contractmeta!(key = "name", val = "lumenscan-testdata");
contractmeta!(key = "description", val = "Fixture exercising every spec entry kind.");
contractmeta!(key = "version", val = "0.1.0");

/// A struct with named fields.
#[contracttype]
#[derive(Clone)]
pub struct Config {
    /// Who may administer the contract.
    pub admin: Address,
    pub fee_bps: u32,
    pub label: String,
}

/// A tuple struct. Its fields are numbered in the spec.
#[contracttype]
#[derive(Clone)]
pub struct Pair(pub i128, pub i128);

/// An enum carrying data, which becomes a union in the spec.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// No payload.
    Admin,
    Balance(Address),
    Allowance(Address, Address),
}

/// A plain integer enum.
#[contracttype]
#[derive(Clone, Copy)]
pub enum Level {
    Low = 0,
    Medium = 1,
    High = 2,
}

/// Contract error codes.
#[contracterror]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum Error {
    NotAuthorized = 1,
    InsufficientBalance = 2,
}

/// An event with one topic and map-formatted data.
#[contractevent]
#[derive(Clone)]
pub struct Transfer {
    #[topic]
    pub from: Address,
    pub to: Address,
    pub amount: i128,
}

#[contract]
pub struct Testdata;

#[contractimpl]
impl Testdata {
    /// Store the configuration. Takes a struct, returns nothing.
    pub fn initialize(env: Env, config: Config) {
        env.storage().instance().set(&DataKey::Admin, &config.admin);
    }

    /// Returns a result, so the spec records both the ok and error types.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InsufficientBalance);
        }
        Transfer { from, to, amount }.publish(&env);
        Ok(())
    }

    /// Deeply nested generics, to exercise the type renderer.
    pub fn nested(items: Option<Vec<Map<Symbol, i128>>>) -> Option<Vec<Map<Symbol, i128>>> {
        items
    }

    /// A tuple return and a fixed-size byte array argument.
    pub fn lookup(key: BytesN<32>) -> (u32, Address) {
        panic!("never called: {key:?}")
    }

    /// The scalar types that have no Rust-native equivalent.
    pub fn scalars(
        when: Timepoint,
        window: Duration,
        blob: Bytes,
        level: Level,
    ) -> (Timepoint, Duration, Bytes, Level) {
        (when, window, blob, level)
    }

    /// Returns a user-defined tuple struct.
    pub fn pair(left: i128, right: i128) -> Pair {
        Pair(left, right)
    }

    /// Takes a union value.
    pub fn describe(key: DataKey) -> Symbol {
        match key {
            DataKey::Admin => Symbol::new(&Env::default(), "admin"),
            _ => Symbol::new(&Env::default(), "other"),
        }
    }
}
