//! Guards the `--json` contract documented in the README.
//!
//! Other tools consume this shape, so a change that breaks these assertions
//! needs a `SCHEMA_VERSION` bump.

use lumenscan::output::{Json, Renderer};
use lumenscan::{Inspection, Source, SCHEMA_VERSION};
use serde_json::Value;

mod fixtures;
use fixtures::testdata;

fn inspect() -> Inspection {
    let source = Source::Wasm {
        path: "testdata.wasm".into(),
    };
    Inspection::from_wasm(source, testdata()).expect("fixture should decode")
}

fn document() -> Value {
    let inspection = inspect();
    let rendered = Json
        .interface(&inspection.interface, &inspection.meta, &inspection.source)
        .expect("rendering should succeed");
    serde_json::from_str(&rendered).expect("output should be valid JSON")
}

#[test]
fn document_is_versioned_and_identifies_its_source() {
    let value = document();

    assert_eq!(value["schema_version"], SCHEMA_VERSION);
    assert_eq!(value["source"]["kind"], "wasm");
    assert_eq!(value["source"]["path"], "testdata.wasm");
}

#[test]
fn every_collection_is_always_present() {
    let value = document();

    // Consumers can index these unconditionally, even for a contract that
    // declares none of them.
    for key in [
        "functions",
        "structs",
        "unions",
        "enums",
        "error_enums",
        "events",
    ] {
        assert!(value[key].is_array(), "`{key}` should always be an array");
    }
}

#[test]
fn functions_carry_named_typed_arguments() {
    let value = document();
    let functions = value["functions"].as_array().unwrap();

    let transfer = functions
        .iter()
        .find(|f| f["name"] == "transfer")
        .expect("transfer function");

    assert_eq!(transfer["inputs"][0]["name"], "from");
    // The field is `type`, not Rust's `type_`.
    assert_eq!(transfer["inputs"][0]["type"], "Address");
    assert_eq!(transfer["outputs"][0], "Result<(), Error>");
}

#[test]
fn user_defined_types_are_reported_with_their_fields() {
    let value = document();

    let config = value["structs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Config")
        .expect("Config struct");
    assert_eq!(config["fields"][0]["name"], "admin");
    assert_eq!(config["fields"][0]["type"], "Address");

    let union = &value["unions"][0];
    assert_eq!(union["name"], "DataKey");
    let allowance = union["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Allowance")
        .unwrap();
    assert_eq!(allowance["types"][0], "Address");

    assert_eq!(value["enums"][0]["cases"][0]["value"], 0);
    assert_eq!(value["error_enums"][0]["cases"][0]["name"], "NotAuthorized");
}

#[test]
fn events_report_topics_and_data_format() {
    let value = document();
    let event = &value["events"][0];

    assert_eq!(event["name"], "Transfer");
    assert_eq!(event["prefix_topics"][0], "transfer");
    assert_eq!(event["data_format"], "map");
    assert_eq!(event["params"][0]["location"], "topic_list");
}

#[test]
fn interface_document_embeds_metadata() {
    let value = document();

    let entries = value["meta"]["entries"].as_array().unwrap();
    assert!(entries
        .iter()
        .any(|e| e["key"] == "name" && e["value"] == "lumenscan-testdata"));
    assert!(value["meta"]["env_interface_version"]["protocol"].is_number());
}

#[test]
fn meta_document_is_self_describing() {
    let inspection = inspect();
    let rendered = Json
        .meta(&inspection.meta, &inspection.source)
        .expect("rendering should succeed");
    let value: Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(value["schema_version"], SCHEMA_VERSION);
    assert_eq!(value["source"]["kind"], "wasm");
    assert!(value["meta"]["entries"].is_array());
}

#[test]
fn empty_docs_are_omitted_rather_than_emitted_as_empty_strings() {
    let value = document();
    let functions = value["functions"].as_array().unwrap();

    let scalars = functions.iter().find(|f| f["name"] == "scalars").unwrap();
    // `scalars` has a doc comment but its parameters do not.
    assert!(scalars["doc"].is_string());
    assert!(
        scalars["inputs"][0].get("doc").is_none(),
        "undocumented parameters should omit the key entirely"
    );
}
