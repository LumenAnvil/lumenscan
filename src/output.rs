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

/// Markdown output for `--format markdown`.
///
/// Renders the same information as [`Human`], but as a document you can paste
/// into a README, an issue, or generated API docs: a heading per section,
/// fenced code for function signatures, and tables for fields and metadata.
#[derive(Debug, Default, Clone, Copy)]
pub struct Markdown;

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

impl Renderer for Markdown {
    fn interface(&self, interface: &Interface, meta: &Meta, source: &Source) -> Result<String> {
        let mut out = String::new();
        let _ = writeln!(out, "# {source}");

        if interface.is_empty() {
            let _ = writeln!(out, "\nThis contract declares no functions or types.");
        }

        md_functions(&mut out, &interface.functions);
        md_structs(&mut out, &interface.structs);
        md_unions(&mut out, &interface.unions);
        md_enums(&mut out, "Enums", &interface.enums);
        md_error_enums(&mut out, &interface.error_enums);
        md_events(&mut out, &interface.events);
        md_meta(&mut out, meta);

        Ok(out)
    }

    fn meta(&self, meta: &Meta, source: &Source) -> Result<String> {
        let mut out = String::new();
        let _ = writeln!(out, "# {source}");

        if meta.is_empty() {
            let _ = writeln!(out, "\nThis contract embeds no metadata.");
        } else {
            md_meta(&mut out, meta);
        }

        Ok(out)
    }
}

/// Write a `## Title (count)` Markdown heading preceded by a blank line.
fn md_section(out: &mut String, title: &str, count: usize) {
    let _ = writeln!(out, "\n## {title} ({count})");
}

/// Collapse a doc comment to a single paragraph: trimmed, blank lines dropped,
/// and pipes escaped so it is safe to place inside a table cell.
fn md_inline(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}

/// Write a doc comment as its own paragraph, skipping it when empty.
fn md_paragraph(out: &mut String, text: &str) {
    let paragraph = md_inline(text);
    if !paragraph.is_empty() {
        let _ = writeln!(out, "\n{paragraph}");
    }
}

fn md_functions(out: &mut String, functions: &[Function]) {
    if functions.is_empty() {
        return;
    }
    md_section(out, "Functions", functions.len());

    for function in functions {
        let args: Vec<String> = function
            .inputs
            .iter()
            .map(|input| format!("{}: {}", input.name, input.ty))
            .collect();
        let returns = match function.outputs.first() {
            Some(ty) => format!(" -> {ty}"),
            None => String::new(),
        };
        let _ = writeln!(
            out,
            "\n### `{}({}){}`",
            function.name,
            args.join(", "),
            returns
        );
        md_paragraph(out, &function.doc);
    }
}

fn md_structs(out: &mut String, structs: &[Struct]) {
    if structs.is_empty() {
        return;
    }
    md_section(out, "Structs", structs.len());

    for item in structs {
        let _ = writeln!(out, "\n### `{}`", item.name);
        md_paragraph(out, &item.doc);
        let rows = item.fields.iter().map(|field| {
            (
                format!("`{}`", field.name),
                format!("`{}`", field.ty),
                &field.doc,
            )
        });
        md_named_table(out, "Field", "Type", rows);
    }
}

fn md_unions(out: &mut String, unions: &[Union]) {
    if unions.is_empty() {
        return;
    }
    md_section(out, "Unions", unions.len());

    for item in unions {
        let _ = writeln!(out, "\n### `{}`", item.name);
        md_paragraph(out, &item.doc);
        let _ = writeln!(out);
        for case in &item.cases {
            let variant = if case.types.is_empty() {
                format!("`{}`", case.name)
            } else {
                format!("`{}({})`", case.name, case.types.join(", "))
            };
            let note = md_inline(&case.doc);
            if note.is_empty() {
                let _ = writeln!(out, "- {variant}");
            } else {
                let _ = writeln!(out, "- {variant} — {note}");
            }
        }
    }
}

fn md_enums(out: &mut String, title: &str, enums: &[Enum]) {
    if enums.is_empty() {
        return;
    }
    md_section(out, title, enums.len());

    for item in enums {
        let _ = writeln!(out, "\n### `{}`", item.name);
        md_paragraph(out, &item.doc);
        let rows = item.cases.iter().map(|case| {
            (
                format!("`{}`", case.name),
                format!("`{}`", case.value),
                &case.doc,
            )
        });
        md_named_table(out, "Variant", "Value", rows);
    }
}

fn md_error_enums(out: &mut String, error_enums: &[ErrorEnum]) {
    if error_enums.is_empty() {
        return;
    }
    md_section(out, "Error enums", error_enums.len());

    for item in error_enums {
        let _ = writeln!(out, "\n### `{}`", item.name);
        md_paragraph(out, &item.doc);
        let rows = item.cases.iter().map(|case| {
            (
                format!("`{}`", case.name),
                format!("`{}`", case.value),
                &case.doc,
            )
        });
        md_named_table(out, "Variant", "Value", rows);
    }
}

fn md_events(out: &mut String, events: &[Event]) {
    if events.is_empty() {
        return;
    }
    md_section(out, "Events", events.len());

    for event in events {
        let _ = writeln!(out, "\n### `{}`", event.name);
        md_paragraph(out, &event.doc);
        let _ = writeln!(out);
        if !event.prefix_topics.is_empty() {
            let topics: Vec<String> = event
                .prefix_topics
                .iter()
                .map(|topic| format!("`{topic}`"))
                .collect();
            let _ = writeln!(out, "- Topics: {}", topics.join(", "));
        }
        let _ = writeln!(out, "- Data format: `{}`", event.data_format);

        let rows = event.params.iter().map(|param| {
            (
                format!("`{}`", param.name),
                format!("`{}` ({})", param.ty, param.location),
                &param.doc,
            )
        });
        md_named_table(out, "Parameter", "Type", rows);
    }
}

fn md_meta(out: &mut String, meta: &Meta) {
    if meta.is_empty() {
        return;
    }
    md_section(out, "Meta", meta.entries.len());
    let _ = writeln!(out);

    if let Some(version) = meta.env_interface_version {
        let _ = writeln!(
            out,
            "- env interface version: protocol {}, pre-release {}",
            version.protocol, version.pre_release
        );
    }

    let _ = writeln!(out, "\n| Key | Value |");
    let _ = writeln!(out, "| --- | --- |");
    for entry in &meta.entries {
        let _ = writeln!(out, "| `{}` | {} |", entry.key, md_inline(&entry.value));
    }
}

/// Write a two- or three-column table of `(name, value, doc)` rows.
///
/// The `Description` column appears only when at least one row is documented,
/// so undocumented types produce a clean two-column table rather than a wall of
/// empty cells.
fn md_named_table<'a>(
    out: &mut String,
    name_header: &str,
    value_header: &str,
    rows: impl Iterator<Item = (String, String, &'a String)> + Clone,
) {
    let has_docs = rows.clone().any(|(_, _, doc)| !md_inline(doc).is_empty());
    let _ = writeln!(out);
    if has_docs {
        let _ = writeln!(out, "| {name_header} | {value_header} | Description |");
        let _ = writeln!(out, "| --- | --- | --- |");
        for (name, value, doc) in rows {
            let _ = writeln!(out, "| {name} | {value} | {} |", md_inline(doc));
        }
    } else {
        let _ = writeln!(out, "| {name_header} | {value_header} |");
        let _ = writeln!(out, "| --- | --- |");
        for (name, value, _) in rows {
            let _ = writeln!(out, "| {name} | {value} |");
        }
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
    fn markdown_output_renders_headings_signatures_and_tables() {
        let rendered = Markdown
            .interface(&interface(), &meta(), &source())
            .unwrap();

        // Title and section headings.
        assert!(rendered.contains("# token.wasm"));
        assert!(rendered.contains("## Functions (1)"));
        // A function is a heading with its full signature as inline code.
        assert!(rendered.contains("### `transfer(to: Address, amount: i128) -> Result<(), Error>`"));
        // Its doc comment becomes a paragraph, not a `#` line.
        assert!(rendered.contains("\nMove tokens."));
        assert!(!rendered.contains("# Move tokens."));
        // The error enum is rendered as a Markdown table.
        assert!(rendered.contains("## Error enums (1)"));
        assert!(rendered.contains("| Variant | Value |"));
        assert!(rendered.contains("| `NotAuthorized` | `1` |"));
        // Meta is a table too.
        assert!(rendered.contains("| `rssdkver` | 27.0.5 |"));
        // Empty sections are omitted entirely.
        assert!(!rendered.contains("## Structs"));
    }

    #[test]
    fn markdown_output_is_explicit_about_an_empty_interface() {
        let rendered = Markdown
            .interface(&Interface::default(), &Meta::default(), &source())
            .unwrap();
        assert!(rendered.contains("# token.wasm"));
        assert!(rendered.contains("declares no functions or types"));
    }

    #[test]
    fn markdown_meta_document_stands_alone() {
        let rendered = Markdown.meta(&meta(), &source()).unwrap();
        assert!(rendered.starts_with("# token.wasm"));
        assert!(rendered.contains("## Meta (1)"));
        assert!(rendered.contains("protocol 27, pre-release 0"));
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
