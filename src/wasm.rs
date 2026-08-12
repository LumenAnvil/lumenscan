//! Extraction of the Soroban custom sections from a WASM module.
//!
//! A Soroban contract carries its interface and metadata in WASM custom
//! sections. Per the Stellar docs, a contract must contain `contractspecv0`
//! (a serialized stream of `SCSpecEntry`) and `contractenvmetav0`, and may
//! contain `contractmetav0`.

use wasmparser::{Parser, Payload};

use crate::error::{Error, Result};

/// Interface spec: a stream of `SCSpecEntry`.
pub const SPEC_SECTION: &str = "contractspecv0";
/// Contract-supplied metadata: a stream of `SCMetaEntry`.
pub const META_SECTION: &str = "contractmetav0";
/// Host environment metadata: a stream of `SCEnvMetaEntry`.
pub const ENV_META_SECTION: &str = "contractenvmetav0";

/// The Soroban custom sections found in a WASM module.
///
/// Each is optional here so that callers can produce their own diagnostics;
/// [`CustomSections::spec_or_err`] enforces that the spec section exists.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CustomSections {
    pub spec: Option<Vec<u8>>,
    pub meta: Option<Vec<u8>>,
    pub env_meta: Option<Vec<u8>>,
}

impl CustomSections {
    /// The interface spec section, or [`Error::MissingSpecSection`].
    pub fn spec_or_err(&self) -> Result<&[u8]> {
        self.spec.as_deref().ok_or(Error::MissingSpecSection)
    }
}

/// Extract the Soroban custom sections from WASM bytes.
///
/// Payloads other than custom sections are ignored, so this keeps working as
/// new WASM features appear.
pub fn custom_sections(wasm: &[u8]) -> Result<CustomSections> {
    let mut found = CustomSections::default();

    for payload in Parser::new(0).parse_all(wasm) {
        if let Payload::CustomSection(reader) = payload? {
            let slot = match reader.name() {
                SPEC_SECTION => &mut found.spec,
                META_SECTION => &mut found.meta,
                ENV_META_SECTION => &mut found.env_meta,
                _ => continue,
            };
            // A well-formed module has one of each; if a module repeats a
            // section, keep the first so behaviour is deterministic.
            slot.get_or_insert_with(|| reader.data().to_vec());
        }
    }

    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest valid module: the 8-byte header, no sections.
    const EMPTY_MODULE: &[u8] = b"\0asm\x01\0\0\0";

    #[test]
    fn finds_nothing_in_a_module_without_soroban_sections() {
        let sections = custom_sections(EMPTY_MODULE).unwrap();
        assert_eq!(sections, CustomSections::default());
        assert!(matches!(
            sections.spec_or_err(),
            Err(Error::MissingSpecSection)
        ));
    }

    #[test]
    fn rejects_bytes_that_are_not_wasm() {
        assert!(matches!(
            custom_sections(b"definitely not wasm"),
            Err(Error::InvalidWasm(_))
        ));
    }

    #[test]
    fn extracts_a_custom_section_by_name() {
        let wasm = module_with_custom_section(SPEC_SECTION, b"payload");
        let sections = custom_sections(&wasm).unwrap();
        assert_eq!(sections.spec.as_deref(), Some(&b"payload"[..]));
        assert_eq!(sections.meta, None);
    }

    #[test]
    fn ignores_unrelated_custom_sections() {
        let wasm = module_with_custom_section("producers", b"rustc");
        assert_eq!(custom_sections(&wasm).unwrap(), CustomSections::default());
    }

    /// Build a module containing a single custom section (section id 0).
    fn module_with_custom_section(name: &str, data: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        leb128(name.len(), &mut body);
        body.extend_from_slice(name.as_bytes());
        body.extend_from_slice(data);

        let mut wasm = EMPTY_MODULE.to_vec();
        wasm.push(0);
        leb128(body.len(), &mut wasm);
        wasm.extend_from_slice(&body);
        wasm
    }

    fn leb128(mut value: usize, out: &mut Vec<u8>) {
        loop {
            let byte = u8::try_from(value & 0x7f).expect("masked to 7 bits");
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }
}
