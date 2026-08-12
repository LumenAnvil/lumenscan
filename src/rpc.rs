//! Fetching a deployed contract's WASM over Stellar RPC.
//!
//! Fetching sits behind the [`WasmFetcher`] trait so that decoding can be
//! exercised against a fixture with no network involved. Tests that need a
//! contract's bytes implement the trait over a byte slice.
//!
//! The network path uses two `getLedgerEntries` calls, which is the sequence
//! Stellar's own tooling performs:
//!
//! 1. read the contract *instance* entry to learn its WASM hash, then
//! 2. read the contract *code* entry at that hash to get the bytes.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use stellar_xdr::{
    ContractDataDurability, ContractExecutable, ContractId, LedgerEntryData, LedgerKey,
    LedgerKeyContractCode, LedgerKeyContractData, Limits, ReadXdr, ScAddress, ScVal, WriteXdr,
};

use crate::error::{Error, Result};

/// Re-exported so implementors of [`WasmFetcher`] need not depend on
/// `stellar-xdr` directly.
pub use stellar_xdr::ContractId as Contract;

/// Public Stellar testnet RPC, used when no URL is supplied.
pub const DEFAULT_RPC_URL: &str = "https://soroban-testnet.stellar.org";

/// How long to wait on an RPC request before giving up.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Source of a contract's WASM bytes.
///
/// Implement this to inspect contracts from somewhere other than Stellar RPC —
/// a local cache, an archive node, or a test fixture.
pub trait WasmFetcher {
    /// Fetch the WASM for a deployed contract.
    fn fetch_wasm(&self, contract: &ContractId) -> Result<Vec<u8>>;
}

/// Parse a `C…` contract strkey.
pub fn parse_contract_id(value: &str) -> Result<ContractId> {
    value
        .parse()
        .map_err(|_| Error::InvalidContractId(value.to_owned()))
}

/// Fetches WASM from a Stellar RPC endpoint over JSON-RPC.
#[derive(Debug, Clone)]
pub struct RpcFetcher {
    url: String,
    agent: ureq::Agent,
}

impl RpcFetcher {
    /// Build a fetcher pointed at an RPC endpoint.
    pub fn new(url: impl Into<String>) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build();
        RpcFetcher {
            url: url.into(),
            agent: config.into(),
        }
    }

    /// The endpoint this fetcher talks to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Call `getLedgerEntries` and return the base64 XDR of each entry found.
    ///
    /// Missing entries are simply absent from the response, so an empty result
    /// means "not on this network" rather than an error.
    fn get_ledger_entries(&self, keys: &[LedgerKey]) -> Result<Vec<String>> {
        let encoded: Vec<String> = keys
            .iter()
            .map(|key| key.to_xdr_base64(Limits::none()))
            .collect::<std::result::Result<_, _>>()
            .map_err(Error::xdr("ledger key"))?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "getLedgerEntries",
            params: LedgerEntriesParams { keys: encoded },
        };

        let response: JsonRpcResponse = self
            .agent
            .post(&self.url)
            .send_json(&request)
            .map_err(|source| Error::Rpc {
                url: self.url.clone(),
                source: Box::new(source),
            })?
            .body_mut()
            .read_json()
            .map_err(|source| Error::Rpc {
                url: self.url.clone(),
                source: Box::new(source),
            })?;

        if let Some(error) = response.error {
            return Err(Error::RpcResponse {
                code: error.code,
                message: error.message,
            });
        }

        Ok(response
            .result
            .and_then(|result| result.entries)
            .unwrap_or_default()
            .into_iter()
            .map(|entry| entry.xdr)
            .collect())
    }

    /// Read the contract instance entry to find its WASM hash.
    fn code_hash(&self, contract: &ContractId) -> Result<stellar_xdr::Hash> {
        let key = LedgerKey::ContractData(LedgerKeyContractData {
            contract: ScAddress::Contract(contract.clone()),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Persistent,
        });

        let entries = self.get_ledger_entries(&[key])?;
        let raw = entries
            .first()
            .ok_or_else(|| Error::ContractNotFound(contract.to_string()))?;

        let entry = LedgerEntryData::from_xdr_base64(raw, Limits::none())
            .map_err(Error::xdr("contract instance entry"))?;

        let LedgerEntryData::ContractData(data) = entry else {
            return Err(Error::UnexpectedLedgerEntry(
                "expected a contract data entry".to_owned(),
            ));
        };
        let ScVal::ContractInstance(instance) = data.val else {
            return Err(Error::UnexpectedLedgerEntry(
                "contract data entry did not hold a contract instance".to_owned(),
            ));
        };

        match instance.executable {
            ContractExecutable::Wasm(hash) => Ok(hash),
            ContractExecutable::StellarAsset => {
                Err(Error::StellarAssetContract(contract.to_string()))
            }
            ContractExecutable::ExternalRef(_) => Err(Error::UnexpectedLedgerEntry(
                "contract runs from an external reference and has no WASM to inspect".to_owned(),
            )),
        }
    }

    /// Read the contract code entry at a WASM hash.
    fn code(&self, hash: &stellar_xdr::Hash) -> Result<Vec<u8>> {
        let key = LedgerKey::ContractCode(LedgerKeyContractCode { hash: hash.clone() });

        let entries = self.get_ledger_entries(&[key])?;
        let raw = entries
            .first()
            .ok_or_else(|| Error::CodeNotFound(hash.to_string()))?;

        let entry = LedgerEntryData::from_xdr_base64(raw, Limits::none())
            .map_err(Error::xdr("contract code entry"))?;

        match entry {
            LedgerEntryData::ContractCode(code) => Ok(code.code.into()),
            _ => Err(Error::UnexpectedLedgerEntry(
                "expected a contract code entry".to_owned(),
            )),
        }
    }
}

impl WasmFetcher for RpcFetcher {
    fn fetch_wasm(&self, contract: &ContractId) -> Result<Vec<u8>> {
        let hash = self.code_hash(contract)?;
        self.code(&hash)
    }
}

#[derive(Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'a str,
    id: u32,
    method: &'a str,
    params: LedgerEntriesParams,
}

#[derive(Serialize)]
struct LedgerEntriesParams {
    keys: Vec<String>,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<LedgerEntriesResult>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct LedgerEntriesResult {
    entries: Option<Vec<LedgerEntryResult>>,
}

#[derive(Deserialize)]
struct LedgerEntryResult {
    xdr: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const NATIVE_TESTNET_SAC: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

    #[test]
    fn parses_a_contract_strkey() {
        assert!(parse_contract_id(NATIVE_TESTNET_SAC).is_ok());
    }

    #[test]
    fn rejects_things_that_are_not_contract_ids() {
        // An account strkey is a valid strkey, but not a contract.
        for value in [
            "not-a-key",
            "",
            "GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ",
        ] {
            assert!(
                matches!(parse_contract_id(value), Err(Error::InvalidContractId(_))),
                "{value} should be rejected"
            );
        }
    }

    #[test]
    fn round_trips_a_contract_id_through_display() {
        let id = parse_contract_id(NATIVE_TESTNET_SAC).unwrap();
        assert_eq!(id.to_string(), NATIVE_TESTNET_SAC);
    }
}
