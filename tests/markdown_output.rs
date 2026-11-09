//! Exercises the `--format markdown` renderer against the committed fixture.
//!
//! The JSON output is a versioned contract with its own guard in
//! `json_output.rs`; Markdown is a human-facing format, so these tests pin the
//! document's structure (headings, fenced signatures, tables) rather than an
//! exact byte layout.

use lumenscan::output::{Markdown, Renderer};
use lumenscan::{Inspection, Source};

mod fixtures;
use fixtures::testdata;

fn render() -> String {
    let source = Source::Wasm {
        path: "testdata.wasm".into(),
    };
    let inspection = Inspection::from_wasm(source, testdata()).expect("fixture should decode");
    Markdown
        .interface(&inspection.interface, &inspection.meta, &inspection.source)
        .expect("rendering should succeed")
}

#[test]
fn document_opens_with_a_title_for_its_source() {
    assert!(render().starts_with("# testdata.wasm"));
}

#[test]
fn each_declared_kind_becomes_a_section_heading() {
    let md = render();
    for heading in [
        "## Functions",
        "## Structs",
        "## Unions",
        "## Enums",
        "## Error enums",
        "## Events",
        "## Meta",
    ] {
        assert!(md.contains(heading), "expected a `{heading}` section");
    }
}

#[test]
fn a_function_is_a_heading_with_its_signature_as_inline_code() {
    let md = render();
    assert!(
        md.contains("### `transfer(from: Address"),
        "transfer should render as a heading with an inline-code signature"
    );
    assert!(
        md.contains("-> Result<(), Error>`"),
        "the return type belongs inside the same inline-code span"
    );
}

#[test]
fn struct_fields_render_as_a_table() {
    let md = render();
    assert!(md.contains("### `Config`"));
    // A GitHub-flavored Markdown table: header, separator, then `name` | `type`.
    assert!(md.contains("| `admin` | `Address`"));
}

#[test]
fn error_variants_render_with_their_discriminants() {
    let md = render();
    assert!(md.contains("| Variant | Value |"));
    assert!(md.contains("| `NotAuthorized` |"));
}

#[test]
fn metadata_renders_as_a_key_value_table() {
    let md = render();
    assert!(md.contains("| Key | Value |"));
    assert!(md.contains("| `name` | lumenscan-testdata |"));
}

#[test]
fn output_is_valid_utf8_without_the_human_comment_prefix() {
    let md = render();
    // The human renderer prefixes doc comments with `# `; the Markdown renderer
    // must not, or every doc line would read as a top-level heading.
    assert!(
        !md.contains("\n# "),
        "doc comments must not be emitted as `# ` heading lines"
    );
}
