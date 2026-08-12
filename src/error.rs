use std::path::PathBuf;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Every way inspection can fail.
///
/// Variants are deliberately specific: a user pointing Lumenscan at a Stellar
/// Asset Contract should be told exactly that, not handed a decode failure.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    ReadWasmFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("not valid WebAssembly: {0}")]
    InvalidWasm(#[from] wasmparser::BinaryReaderError),

    #[error(
        "no `contractspecv0` custom section found; this WASM does not look like a Soroban contract"
    )]
    MissingSpecSection,

    #[error("failed to decode the `{section}` section as XDR: {source}")]
    Xdr {
        section: &'static str,
        #[source]
        source: stellar_xdr::Error,
    },

    #[error("`{0}` is not a valid contract ID (expected a strkey starting with `C`)")]
    InvalidContractId(String),

    #[error("RPC request to {url} failed: {source}")]
    Rpc {
        url: String,
        #[source]
        source: Box<ureq::Error>,
    },

    #[error("RPC returned an error: {message} (code {code})")]
    RpcResponse { code: i64, message: String },

    #[error("contract {0} was not found on this network (check --rpc-url)")]
    ContractNotFound(String),

    #[error("no WASM found on the network for code hash {0}")]
    CodeNotFound(String),

    #[error(
        "contract {0} is a Stellar Asset Contract; it is implemented by the host and has no WASM to inspect"
    )]
    StellarAssetContract(String),

    #[error("unexpected ledger entry shape returned by RPC: {0}")]
    UnexpectedLedgerEntry(String),

    #[error("failed to serialize output as JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    /// Attach the section name to an XDR decode failure.
    pub(crate) fn xdr(section: &'static str) -> impl Fn(stellar_xdr::Error) -> Self {
        move |source| Error::Xdr { section, source }
    }
}
