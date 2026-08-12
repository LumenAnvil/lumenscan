//! Lumenscan inspects a deployed Soroban contract's interface.
//!
//! A contract's functions, argument names and types, user-defined types, and
//! metadata all live inside its WASM, in custom sections holding XDR. This
//! crate fetches that WASM (or reads it from disk), pulls out those sections,
//! decodes them, and renders the result.
//!
//! The pipeline is three steps, each usable on its own:
//!
//! 1. [`rpc`] — fetch WASM bytes for a contract ID, behind the
//!    [`WasmFetcher`](rpc::WasmFetcher) trait so the rest is testable offline.
//! 2. [`wasm`] + [`spec`] — extract and decode the custom sections into an
//!    [`Interface`](spec::Interface) and [`Meta`](spec::Meta).
//! 3. [`output`] — render via the [`Renderer`](output::Renderer) trait.
//!
//! ```no_run
//! use lumenscan::{spec, wasm};
//!
//! let bytes = std::fs::read("contract.wasm")?;
//! let sections = wasm::custom_sections(&bytes)?;
//! let interface = spec::decode_interface(sections.spec_or_err()?)?;
//! println!("{} functions", interface.functions.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod error;
pub mod output;
pub mod rpc;
pub mod spec;
pub mod wasm;

use std::path::{Path, PathBuf};

use serde::Serialize;

pub use error::{Error, Result};

/// Version of the `--json` document shape.
///
/// Bumped only on a breaking change to the output, so tools can rely on it.
pub const SCHEMA_VERSION: u32 = 1;

/// Where an inspected contract came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    /// Fetched from the network by contract ID.
    Contract { id: String },
    /// Read from a local file.
    Wasm { path: PathBuf },
}

impl Source {
    /// Read WASM from a local file, pairing IO errors with the path.
    pub fn read_wasm_file(path: &Path) -> Result<(Self, Vec<u8>)> {
        let bytes = std::fs::read(path).map_err(|source| Error::ReadWasmFile {
            path: path.to_path_buf(),
            source,
        })?;
        Ok((
            Source::Wasm {
                path: path.to_path_buf(),
            },
            bytes,
        ))
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Contract { id } => write!(f, "contract {id}"),
            Source::Wasm { path } => write!(f, "{}", path.display()),
        }
    }
}

/// Everything Lumenscan decoded from one contract's WASM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    pub source: Source,
    pub interface: spec::Interface,
    pub meta: spec::Meta,
}

impl Inspection {
    /// Decode WASM bytes into an [`Inspection`].
    pub fn from_wasm(source: Source, wasm_bytes: &[u8]) -> Result<Self> {
        let sections = wasm::custom_sections(wasm_bytes)?;
        Ok(Inspection {
            source,
            interface: spec::decode_interface(sections.spec_or_err()?)?,
            meta: spec::decode_meta(sections.meta.as_deref(), sections.env_meta.as_deref())?,
        })
    }

    /// Decode only the metadata sections.
    ///
    /// Unlike [`Inspection::from_wasm`] this tolerates a missing spec section,
    /// so `lumenscan meta` still works on a non-Soroban module.
    pub fn meta_from_wasm(source: Source, wasm_bytes: &[u8]) -> Result<(Source, spec::Meta)> {
        let sections = wasm::custom_sections(wasm_bytes)?;
        let meta = spec::decode_meta(sections.meta.as_deref(), sections.env_meta.as_deref())?;
        Ok((source, meta))
    }
}
