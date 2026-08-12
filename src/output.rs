//! Rendering of a decoded interface.
//!
//! Formats sit behind the [`Renderer`] trait. Adding one (say YAML, or a
//! Markdown table) means writing one more implementation and wiring it into the
//! CLI's format flag — nothing else in the crate needs to change.

use std::fmt::Write as _;

use serde::Serialize;

use crate::error::Result;
use crate::spec::{Enum, ErrorEnum, Event, Function, Interface, Meta, Struct, Union};
use crate::{Source, SCHEMA_VERSION};

/// Renders decoded contract data as text.
pub trait Renderer {
    /// Render a contract's full interface, including its metadata.
    fn interface(&self, interface: &Interface, meta: &Meta, source: &Source) -> Result<String>;

    /// Render only a contract's metadata.
    fn meta(&self, meta: &Meta, source: &Source) -> Result<String>;
}

/// Human-readable output: the default.
#[derive(Debug, Default, Clone, Copy)]
pub struct Human;

/// Machine-readable output for `--json`.
///
/// The shape is documented in the README and versioned by [`SCHEMA_VERSION`].
#[derive(Debug, Default, Clone, Copy)]
pub struct Json;

/// The `--json` document for a full inspection.
#[derive(Serialize)]
struct InterfaceDocument<'a> {
    schema_version: u32,
    source: &'a Source,
    #[serde(flatten)]
    interface: &'a Interface,
    meta: &'a Meta,
}

/// The `--json` document for `lumenscan meta`.
#[derive(Serialize)]
struct MetaDocument<'a> {
    schema_version: u32,
    source: &'a Source,
    meta: &'a Meta,
}

impl Renderer for Json {
    fn interface(&self, interface: &Interface, meta: &Meta, source: &Source) -> Result<String> {
        let document = InterfaceDocument {
            schema_version: SCHEMA_VERSION,
            source,
            interface,
            meta,
        };
        Ok(serde_json::to_string_pretty(&document)?)
    }

    fn meta(&self, meta: &Meta, source: &Source) -> Result<String> {
        let document = MetaDocument {
            schema_version: SCHEMA_VERSION,
            source,
            meta,
        };
        Ok(serde_json::to_string_pretty(&document)?)
    }
}

impl Renderer for Human {
    fn interface(&self, interface: &Interface, meta: &Meta, source: &Source) -> Result<String> {
        let mut out = String::new();
        let _ = writeln!(out, "{source}");

        if interface.is_empty() {
            let _ = writeln!(out, "\nThis contract declares no functions or types.");
        }

        write_functions(&mut out, &interface.functions);
        write_structs(&mut out, &interface.structs);
        write_unions(&mut out, &interface.unions);
        write_enums(&mut out, "Enums", &interface.enums);
        write_error_enums(&mut out, &interface.error_enums);
        write_events(&mut out, &interface.events);
        write_meta(&mut out, meta);

        Ok(out)
    }

    fn meta(&self, meta: &Meta, source: &Source) -> Result<String> {
        let mut out = String::new();
        let _ = writeln!(out, "{source}");

        if meta.is_empty() {
            let _ = writeln!(out, "\nThis contract embeds no metadata.");
        } else {
            write_meta(&mut out, meta);
        }

        Ok(out)
    }
}

/// Write a `Name (count)` section header preceded by a blank line.
fn section(out: &mut String, title: &str, count: usize) {
    let _ = writeln!(out, "\n{title} ({count})");
}

/// Write a doc comment, indented, skipping it when empty.
fn doc(out: &mut String, text: &str, indent: &str) {
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let _ = writeln!(out, "{indent}# {}", line.trim());
    }
}

fn write_functions(out: &mut String, functions: &[Function]) {
    if functions.is_empty() {
        return;
    }
    section(out, "Functions", functions.len());

    for function in functions {
        doc(out, &function.doc, "  ");
        let args: Vec<String> = function
            .inputs
            .iter()
            .map(|input| format!("{}: {}", input.name, input.ty))
            .collect();
        let returns = match function.outputs.first() {
            Some(ty) => format!(" -> {ty}"),
            None => String::new(),
        };
        let _ = writeln!(out, "  {}({}){}", function.name, args.join(", "), returns);
    }
}

fn write_structs(out: &mut String, structs: &[Struct]) {
    if structs.is_empty() {
        return;
    }
    section(out, "Structs", structs.len());

    for item in structs {
        doc(out, &item.doc, "  ");
        let _ = writeln!(out, "  {}", item.name);
        for field in &item.fields {
            doc(out, &field.doc, "    ");
            let _ = writeln!(out, "    {}: {}", field.name, field.ty);
        }
    }
}

fn write_unions(out: &mut String, unions: &[Union]) {
    if unions.is_empty() {
        return;
    }
    section(out, "Unions", unions.len());

    for item in unions {
        doc(out, &item.doc, "  ");
        let _ = writeln!(out, "  {}", item.name);
        for case in &item.cases {
            doc(out, &case.doc, "    ");
            if case.types.is_empty() {
                let _ = writeln!(out, "    {}", case.name);
            } else {
                let _ = writeln!(out, "    {}({})", case.name, case.types.join(", "));
            }
        }
    }
}

fn write_enums(out: &mut String, title: &str, enums: &[Enum]) {
    if enums.is_empty() {
        return;
    }
    section(out, title, enums.len());

    for item in enums {
        doc(out, &item.doc, "  ");
        let _ = writeln!(out, "  {}", item.name);
        for case in &item.cases {
            doc(out, &case.doc, "    ");
            let _ = writeln!(out, "    {} = {}", case.name, case.value);
        }
    }
}

fn write_error_enums(out: &mut String, error_enums: &[ErrorEnum]) {
    if error_enums.is_empty() {
        return;
    }
    section(out, "Error enums", error_enums.len());

    for item in error_enums {
        doc(out, &item.doc, "  ");
        let _ = writeln!(out, "  {}", item.name);
        for case in &item.cases {
            doc(out, &case.doc, "    ");
            let _ = writeln!(out, "    {} = {}", case.name, case.value);
        }
    }
}

fn write_events(out: &mut String, events: &[Event]) {
    if events.is_empty() {
        return;
    }
    section(out, "Events", events.len());

    for event in events {
        doc(out, &event.doc, "  ");
        let topics = if event.prefix_topics.is_empty() {
            String::new()
        } else {
            format!(" [topics: {}]", event.prefix_topics.join(", "))
        };
        let _ = writeln!(
            out,
            "  {}{} (data format: {})",
            event.name, topics, event.data_format
        );
        for param in &event.params {
            doc(out, &param.doc, "    ");
            let _ = writeln!(out, "    {}: {} ({})", param.name, param.ty, param.location);
        }
    }
}

fn write_meta(out: &mut String, meta: &Meta) {
    if meta.is_empty() {
        return;
    }
    section(out, "Meta", meta.entries.len());

    if let Some(version) = meta.env_interface_version {
        let _ = writeln!(
            out,
            "  env interface version: protocol {}, pre-release {}",
            version.protocol, version.pre_release
        );
    }

    let width = meta
        .entries
        .iter()
        .map(|entry| entry.key.len())
        .max()
        .unwrap_or(0);
    for entry in &meta.entries {
        let _ = writeln!(out, "  {:<width$}  {}", entry.key, entry.value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{EnumCase, InterfaceVersion, MetaEntry, Param};
    use std::path::PathBuf;

    fn source() -> Source {
        Source::Wasm {
            path: PathBuf::from("token.wasm"),
        }
    }

    fn interface() -> Interface {
        Interface {
            functions: vec![Function {
                name: "transfer".to_owned(),
                doc: "Move tokens.".to_owned(),
                inputs: vec![
                    Param {
                        name: "to".to_owned(),
                        doc: String::new(),
                        ty: "Address".to_owned(),
                    },
                    Param {
                        name: "amount".to_owned(),
                        doc: String::new(),
                        ty: "i128".to_owned(),
                    },
                ],
                outputs: vec!["Result<(), Error>".to_owned()],
            }],
            error_enums: vec![ErrorEnum {
                name: "Error".to_owned(),
                doc: String::new(),
                lib: String::new(),
                cases: vec![EnumCase {
                    name: "NotAuthorized".to_owned(),
                    doc: String::new(),
                    value: 1,
                }],
            }],
            ..Interface::default()
        }
    }

    fn meta() -> Meta {
        Meta {
            entries: vec![MetaEntry {
                key: "rssdkver".to_owned(),
                value: "27.0.5".to_owned(),
            }],
            env_interface_version: Some(InterfaceVersion {
                protocol: 27,
                pre_release: 0,
            }),
        }
    }

    #[test]
    fn human_output_shows_signatures_and_meta() {
        let rendered = Human.interface(&interface(), &meta(), &source()).unwrap();

        assert!(rendered.contains("transfer(to: Address, amount: i128) -> Result<(), Error>"));
        assert!(rendered.contains("# Move tokens."));
        assert!(rendered.contains("NotAuthorized = 1"));
        assert!(rendered.contains("rssdkver"));
        // Sections with no entries are omitted entirely.
        assert!(!rendered.contains("Structs"));
    }

    #[test]
    fn human_output_is_explicit_about_an_empty_interface() {
        let rendered = Human
            .interface(&Interface::default(), &Meta::default(), &source())
            .unwrap();
        assert!(rendered.contains("declares no functions or types"));
    }

    #[test]
    fn json_output_matches_the_documented_shape() {
        let rendered = Json.interface(&interface(), &meta(), &source()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(value["schema_version"], SCHEMA_VERSION);
        assert_eq!(value["source"]["kind"], "wasm");
        assert_eq!(value["source"]["path"], "token.wasm");

        let function = &value["functions"][0];
        assert_eq!(function["name"], "transfer");
        assert_eq!(function["inputs"][0]["name"], "to");
        assert_eq!(function["inputs"][0]["type"], "Address");
        assert_eq!(function["outputs"][0], "Result<(), Error>");

        // Collections are always present, even when empty, so consumers do not
        // have to handle missing keys.
        for key in ["structs", "unions", "enums", "error_enums", "events"] {
            assert!(value[key].is_array(), "{key} should always be an array");
        }

        assert_eq!(value["meta"]["entries"][0]["key"], "rssdkver");
        assert_eq!(value["meta"]["env_interface_version"]["protocol"], 27);
    }

    #[test]
    fn json_meta_document_is_self_describing() {
        let rendered = Json.meta(&meta(), &source()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(value["schema_version"], SCHEMA_VERSION);
        assert_eq!(value["source"]["kind"], "wasm");
        assert_eq!(value["meta"]["entries"][0]["value"], "27.0.5");
    }
}
