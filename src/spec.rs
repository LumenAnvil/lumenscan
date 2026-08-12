//! Decoding of `SCSpecEntry` / `SCMetaEntry` XDR into a clean interface model.
//!
//! The model here is deliberately independent of the `stellar-xdr` types: it is
//! what gets rendered and what `--json` promises. Keeping it separate means an
//! XDR crate upgrade cannot silently reshape the published output.

use std::io::Cursor;

use serde::Serialize;
use stellar_xdr::{
    Limited, Limits, ReadXdr, ScEnvMetaEntry, ScMetaEntry, ScSpecEntry, ScSpecEventDataFormat,
    ScSpecEventParamLocationV0, ScSpecTypeDef, ScSpecUdtUnionCaseV0,
};

use crate::error::{Error, Result};
use crate::wasm::{ENV_META_SECTION, META_SECTION, SPEC_SECTION};

/// A contract's full interface, grouped by kind of declaration.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Interface {
    pub functions: Vec<Function>,
    pub structs: Vec<Struct>,
    pub unions: Vec<Union>,
    pub enums: Vec<Enum>,
    pub error_enums: Vec<ErrorEnum>,
    pub events: Vec<Event>,
}

impl Interface {
    /// True when the contract declares nothing at all.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
            && self.structs.is_empty()
            && self.unions.is_empty()
            && self.enums.is_empty()
            && self.error_enums.is_empty()
            && self.events.is_empty()
    }
}

/// An exported contract function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Function {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    pub inputs: Vec<Param>,
    /// Rendered return types. Soroban allows at most one.
    pub outputs: Vec<String>,
}

/// A named, typed parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Param {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// A user-defined struct. Tuple structs have numeric field names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Struct {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lib: String,
    pub fields: Vec<Param>,
}

/// A user-defined enum with associated data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Union {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lib: String,
    pub cases: Vec<UnionCase>,
}

/// One variant of a [`Union`]; `types` is empty for a void case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnionCase {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    pub types: Vec<String>,
}

/// A C-style enum of `u32` values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Enum {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lib: String,
    pub cases: Vec<EnumCase>,
}

/// A named `u32` value in an [`Enum`] or [`ErrorEnum`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnumCase {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    pub value: u32,
}

/// An enum of contract error codes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorEnum {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lib: String,
    pub cases: Vec<EnumCase>,
}

/// An event the contract declares it can publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Event {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lib: String,
    pub prefix_topics: Vec<String>,
    pub params: Vec<EventParam>,
    /// `single_value`, `vec`, or `map`.
    pub data_format: String,
}

/// A parameter of an [`Event`], carried either in the topics or the data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventParam {
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
    #[serde(rename = "type")]
    pub ty: String,
    /// `data` or `topic_list`.
    pub location: String,
}

/// Contract metadata: the contract's own key/value entries plus the host
/// interface version recorded by the SDK.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Meta {
    pub entries: Vec<MetaEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_interface_version: Option<InterfaceVersion>,
}

impl Meta {
    /// True when the WASM carried no metadata at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.env_interface_version.is_none()
    }
}

/// One `contractmetav0` key/value pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetaEntry {
    pub key: String,
    pub value: String,
}

/// The host interface version from `contractenvmetav0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct InterfaceVersion {
    pub protocol: u32,
    pub pre_release: u32,
}

/// Decode the `contractspecv0` section into an [`Interface`].
///
/// The section is a stream of concatenated `SCSpecEntry` values, so entries are
/// read with [`ReadXdr::read_xdr_iter`] rather than a single `from_xdr`.
pub fn decode_interface(spec: &[u8]) -> Result<Interface> {
    let mut interface = Interface::default();

    for entry in read_stream::<ScSpecEntry>(spec, SPEC_SECTION)? {
        match entry {
            ScSpecEntry::FunctionV0(f) => interface.functions.push(Function {
                name: f.name.0.to_utf8_string_lossy(),
                doc: f.doc.to_utf8_string_lossy(),
                inputs: f
                    .inputs
                    .iter()
                    .map(|i| Param {
                        name: i.name.to_utf8_string_lossy(),
                        doc: i.doc.to_utf8_string_lossy(),
                        ty: type_name(&i.type_),
                    })
                    .collect(),
                outputs: f.outputs.iter().map(type_name).collect(),
            }),
            ScSpecEntry::UdtStructV0(s) => interface.structs.push(Struct {
                name: s.name.to_utf8_string_lossy(),
                doc: s.doc.to_utf8_string_lossy(),
                lib: s.lib.to_utf8_string_lossy(),
                fields: s
                    .fields
                    .iter()
                    .map(|f| Param {
                        name: f.name.to_utf8_string_lossy(),
                        doc: f.doc.to_utf8_string_lossy(),
                        ty: type_name(&f.type_),
                    })
                    .collect(),
            }),
            ScSpecEntry::UdtUnionV0(u) => interface.unions.push(Union {
                name: u.name.to_utf8_string_lossy(),
                doc: u.doc.to_utf8_string_lossy(),
                lib: u.lib.to_utf8_string_lossy(),
                cases: u
                    .cases
                    .iter()
                    .map(|case| match case {
                        ScSpecUdtUnionCaseV0::VoidV0(v) => UnionCase {
                            name: v.name.to_utf8_string_lossy(),
                            doc: v.doc.to_utf8_string_lossy(),
                            types: Vec::new(),
                        },
                        ScSpecUdtUnionCaseV0::TupleV0(t) => UnionCase {
                            name: t.name.to_utf8_string_lossy(),
                            doc: t.doc.to_utf8_string_lossy(),
                            types: t.type_.iter().map(type_name).collect(),
                        },
                    })
                    .collect(),
            }),
            ScSpecEntry::UdtEnumV0(e) => interface.enums.push(Enum {
                name: e.name.to_utf8_string_lossy(),
                doc: e.doc.to_utf8_string_lossy(),
                lib: e.lib.to_utf8_string_lossy(),
                cases: e
                    .cases
                    .iter()
                    .map(|c| EnumCase {
                        name: c.name.to_utf8_string_lossy(),
                        doc: c.doc.to_utf8_string_lossy(),
                        value: c.value,
                    })
                    .collect(),
            }),
            ScSpecEntry::UdtErrorEnumV0(e) => interface.error_enums.push(ErrorEnum {
                name: e.name.to_utf8_string_lossy(),
                doc: e.doc.to_utf8_string_lossy(),
                lib: e.lib.to_utf8_string_lossy(),
                cases: e
                    .cases
                    .iter()
                    .map(|c| EnumCase {
                        name: c.name.to_utf8_string_lossy(),
                        doc: c.doc.to_utf8_string_lossy(),
                        value: c.value,
                    })
                    .collect(),
            }),
            ScSpecEntry::EventV0(e) => interface.events.push(Event {
                name: e.name.0.to_utf8_string_lossy(),
                doc: e.doc.to_utf8_string_lossy(),
                lib: e.lib.to_utf8_string_lossy(),
                prefix_topics: e
                    .prefix_topics
                    .iter()
                    .map(|t| t.0.to_utf8_string_lossy())
                    .collect(),
                params: e
                    .params
                    .iter()
                    .map(|p| EventParam {
                        name: p.name.to_utf8_string_lossy(),
                        doc: p.doc.to_utf8_string_lossy(),
                        ty: type_name(&p.type_),
                        location: match p.location {
                            ScSpecEventParamLocationV0::Data => "data",
                            ScSpecEventParamLocationV0::TopicList => "topic_list",
                        }
                        .to_owned(),
                    })
                    .collect(),
                data_format: match e.data_format {
                    ScSpecEventDataFormat::SingleValue => "single_value",
                    ScSpecEventDataFormat::Vec => "vec",
                    ScSpecEventDataFormat::Map => "map",
                }
                .to_owned(),
            }),
        }
    }

    Ok(interface)
}

/// Decode the `contractmetav0` and `contractenvmetav0` sections.
///
/// Both are optional: a WASM without them yields an empty [`Meta`].
pub fn decode_meta(meta: Option<&[u8]>, env_meta: Option<&[u8]>) -> Result<Meta> {
    let mut decoded = Meta::default();

    if let Some(bytes) = meta {
        for entry in read_stream::<ScMetaEntry>(bytes, META_SECTION)? {
            let ScMetaEntry::ScMetaV0(v0) = entry;
            decoded.entries.push(MetaEntry {
                key: v0.key.to_utf8_string_lossy(),
                value: v0.val.to_utf8_string_lossy(),
            });
        }
    }

    if let Some(bytes) = env_meta {
        for entry in read_stream::<ScEnvMetaEntry>(bytes, ENV_META_SECTION)? {
            let ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(v) = entry;
            decoded.env_interface_version = Some(InterfaceVersion {
                protocol: v.protocol,
                pre_release: v.pre_release,
            });
        }
    }

    Ok(decoded)
}

/// Read a custom section as a stream of concatenated XDR values.
fn read_stream<T: ReadXdr>(bytes: &[u8], section: &'static str) -> Result<Vec<T>> {
    let mut reader = Limited::new(Cursor::new(bytes), Limits::none());
    T::read_xdr_iter(&mut reader)
        .collect::<std::result::Result<Vec<T>, _>>()
        .map_err(Error::xdr(section))
}

/// Render a spec type as the Rust-like name a contract author would recognise.
///
/// Matching is exhaustive on purpose: when a new `ScSpecTypeDef` variant is
/// added upstream, this fails to compile instead of printing something wrong.
pub fn type_name(ty: &ScSpecTypeDef) -> String {
    match ty {
        ScSpecTypeDef::Val => "Val".to_owned(),
        ScSpecTypeDef::Bool => "bool".to_owned(),
        ScSpecTypeDef::Void => "()".to_owned(),
        ScSpecTypeDef::Error => "Error".to_owned(),
        ScSpecTypeDef::U32 => "u32".to_owned(),
        ScSpecTypeDef::I32 => "i32".to_owned(),
        ScSpecTypeDef::U64 => "u64".to_owned(),
        ScSpecTypeDef::I64 => "i64".to_owned(),
        ScSpecTypeDef::Timepoint => "Timepoint".to_owned(),
        ScSpecTypeDef::Duration => "Duration".to_owned(),
        ScSpecTypeDef::U128 => "u128".to_owned(),
        ScSpecTypeDef::I128 => "i128".to_owned(),
        ScSpecTypeDef::U256 => "u256".to_owned(),
        ScSpecTypeDef::I256 => "i256".to_owned(),
        ScSpecTypeDef::Bytes => "Bytes".to_owned(),
        ScSpecTypeDef::String => "String".to_owned(),
        ScSpecTypeDef::Symbol => "Symbol".to_owned(),
        ScSpecTypeDef::Address => "Address".to_owned(),
        ScSpecTypeDef::MuxedAddress => "MuxedAddress".to_owned(),
        ScSpecTypeDef::Option(o) => format!("Option<{}>", type_name(&o.value_type)),
        ScSpecTypeDef::Result(r) => format!(
            "Result<{}, {}>",
            type_name(&r.ok_type),
            type_name(&r.error_type)
        ),
        ScSpecTypeDef::Vec(v) => format!("Vec<{}>", type_name(&v.element_type)),
        ScSpecTypeDef::Map(m) => format!(
            "Map<{}, {}>",
            type_name(&m.key_type),
            type_name(&m.value_type)
        ),
        ScSpecTypeDef::Tuple(t) => {
            let inner: Vec<String> = t.value_types.iter().map(type_name).collect();
            format!("({})", inner.join(", "))
        }
        ScSpecTypeDef::BytesN(b) => format!("BytesN<{}>", b.n),
        ScSpecTypeDef::Udt(u) => u.name.to_utf8_string_lossy(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::{
        ScSpecTypeMap, ScSpecTypeOption, ScSpecTypeResult, ScSpecTypeTuple, ScSpecTypeUdt,
        ScSpecTypeVec, WriteXdr,
    };

    fn udt(name: &str) -> ScSpecTypeDef {
        ScSpecTypeDef::Udt(ScSpecTypeUdt {
            name: name.try_into().unwrap(),
        })
    }

    #[test]
    fn renders_primitive_types() {
        assert_eq!(type_name(&ScSpecTypeDef::U32), "u32");
        assert_eq!(type_name(&ScSpecTypeDef::I128), "i128");
        assert_eq!(type_name(&ScSpecTypeDef::Void), "()");
        assert_eq!(type_name(&ScSpecTypeDef::Address), "Address");
    }

    #[test]
    fn renders_nested_composite_types() {
        let inner = ScSpecTypeDef::Map(Box::new(ScSpecTypeMap {
            key_type: Box::new(ScSpecTypeDef::Symbol),
            value_type: Box::new(ScSpecTypeDef::I128),
        }));
        let vec = ScSpecTypeDef::Vec(Box::new(ScSpecTypeVec {
            element_type: Box::new(inner),
        }));
        let option = ScSpecTypeDef::Option(Box::new(ScSpecTypeOption {
            value_type: Box::new(vec),
        }));

        assert_eq!(type_name(&option), "Option<Vec<Map<Symbol, i128>>>");
    }

    #[test]
    fn renders_result_tuple_and_bytes_n() {
        let result = ScSpecTypeDef::Result(Box::new(ScSpecTypeResult {
            ok_type: Box::new(ScSpecTypeDef::Void),
            error_type: Box::new(udt("TokenError")),
        }));
        assert_eq!(type_name(&result), "Result<(), TokenError>");

        let tuple = ScSpecTypeDef::Tuple(Box::new(ScSpecTypeTuple {
            value_types: vec![ScSpecTypeDef::U32, ScSpecTypeDef::Address]
                .try_into()
                .unwrap(),
        }));
        assert_eq!(type_name(&tuple), "(u32, Address)");

        assert_eq!(
            type_name(&ScSpecTypeDef::BytesN(stellar_xdr::ScSpecTypeBytesN {
                n: 32
            })),
            "BytesN<32>"
        );
    }

    #[test]
    fn decodes_a_stream_of_several_entries() {
        // Two functions written back to back, exactly as the custom section
        // stores them.
        let entries = [spec_function("hello"), spec_function("world")];
        let mut bytes = Vec::new();
        for entry in &entries {
            bytes.extend(entry.to_xdr(Limits::none()).unwrap());
        }

        let interface = decode_interface(&bytes).unwrap();
        let names: Vec<&str> = interface
            .functions
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(names, ["hello", "world"]);
        assert!(interface.structs.is_empty());
    }

    #[test]
    fn reports_the_section_name_when_xdr_is_corrupt() {
        let err = decode_interface(&[0xff, 0xff, 0xff, 0xff]).unwrap_err();
        assert!(matches!(
            err,
            Error::Xdr {
                section: SPEC_SECTION,
                ..
            }
        ));
    }

    #[test]
    fn empty_meta_sections_decode_to_empty_meta() {
        assert!(decode_meta(None, None).unwrap().is_empty());
    }

    fn spec_function(name: &str) -> ScSpecEntry {
        ScSpecEntry::FunctionV0(stellar_xdr::ScSpecFunctionV0 {
            doc: "".try_into().unwrap(),
            name: stellar_xdr::ScSymbol(name.try_into().unwrap()),
            inputs: Vec::new().try_into().unwrap(),
            outputs: Vec::new().try_into().unwrap(),
        })
    }
}
