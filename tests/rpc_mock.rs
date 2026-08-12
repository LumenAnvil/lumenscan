//! Proves the fetching seam works: decoding runs against any [`WasmFetcher`],
//! so nothing below the network boundary needs a network to be tested.

use std::cell::RefCell;

use lumenscan::rpc::{parse_contract_id, Contract as ContractId, WasmFetcher};
use lumenscan::{Error, Inspection, Result, Source};

mod fixtures;
use fixtures::testdata;

const CONTRACT_ID: &str = "CBTBKYT3OMI4LOUX3UGOZTQSGFAABHHNVRVVGI4SSX3WWNHAIGQAE6TC";

/// A fetcher that serves fixture bytes and records what it was asked for.
struct StubFetcher {
    wasm: &'static [u8],
    requested: RefCell<Vec<String>>,
}

impl StubFetcher {
    fn new(wasm: &'static [u8]) -> Self {
        StubFetcher {
            wasm,
            requested: RefCell::new(Vec::new()),
        }
    }
}

impl WasmFetcher for StubFetcher {
    fn fetch_wasm(&self, contract: &ContractId) -> Result<Vec<u8>> {
        self.requested.borrow_mut().push(contract.to_string());
        Ok(self.wasm.to_vec())
    }
}

/// A fetcher standing in for a contract that is not on the network.
struct MissingFetcher;

impl WasmFetcher for MissingFetcher {
    fn fetch_wasm(&self, contract: &ContractId) -> Result<Vec<u8>> {
        Err(Error::ContractNotFound(contract.to_string()))
    }
}

/// The path the CLI takes: fetch by contract ID, then decode.
fn inspect_with(fetcher: &dyn WasmFetcher, id: &str) -> Result<Inspection> {
    let contract = parse_contract_id(id)?;
    let wasm = fetcher.fetch_wasm(&contract)?;
    Inspection::from_wasm(
        Source::Contract {
            id: contract.to_string(),
        },
        &wasm,
    )
}

#[test]
fn decodes_a_contract_fetched_through_the_trait() {
    let fetcher = StubFetcher::new(testdata());

    let inspection = inspect_with(&fetcher, CONTRACT_ID).expect("should decode");

    assert_eq!(inspection.interface.functions.len(), 7);
    assert_eq!(
        inspection.source,
        Source::Contract {
            id: CONTRACT_ID.to_owned(),
        }
    );
    // The fetcher received the parsed ID, round-tripped back to its strkey.
    assert_eq!(fetcher.requested.borrow().as_slice(), [CONTRACT_ID]);
}

#[test]
fn surfaces_a_missing_contract_as_a_clear_error() {
    let error = inspect_with(&MissingFetcher, CONTRACT_ID).unwrap_err();

    assert!(matches!(error, Error::ContractNotFound(id) if id == CONTRACT_ID));
}

#[test]
fn rejects_an_invalid_contract_id_before_any_fetching() {
    let fetcher = StubFetcher::new(testdata());

    let error = inspect_with(&fetcher, "not-a-contract-id").unwrap_err();

    assert!(matches!(error, Error::InvalidContractId(_)));
    assert!(
        fetcher.requested.borrow().is_empty(),
        "an unparseable ID should never reach the network"
    );
}
