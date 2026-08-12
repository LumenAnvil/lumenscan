//! End-to-end decoding against checked-in WASM fixtures.
//!
//! These run entirely offline: the fixtures are committed to the repository, so
//! nothing here touches the network.

use lumenscan::spec::{Interface, Meta};
use lumenscan::{wasm, Inspection, Source};

mod fixtures;
use fixtures::{oracle, testdata};

/// Decode a fixture into its interface and metadata.
fn decode(bytes: &[u8]) -> (Interface, Meta) {
    let source = Source::Wasm {
        path: "fixture.wasm".into(),
    };
    let inspection = Inspection::from_wasm(source, bytes).expect("fixture should decode");
    (inspection.interface, inspection.meta)
}

/// Look up a function by name, failing with a useful message if absent.
fn function<'a>(interface: &'a Interface, name: &str) -> &'a lumenscan::spec::Function {
    interface
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            let names: Vec<&str> = interface
                .functions
                .iter()
                .map(|f| f.name.as_str())
                .collect();
            panic!("no function `{name}`; found {names:?}")
        })
}

#[test]
fn extracts_the_soroban_custom_sections() {
    let sections = wasm::custom_sections(testdata()).unwrap();

    assert!(sections.spec.is_some(), "spec section is required");
    assert!(sections.meta.is_some(), "fixture embeds contractmeta");
    assert!(sections.env_meta.is_some(), "SDK always emits env meta");
}

#[test]
fn decodes_every_kind_of_spec_entry() {
    let (interface, _) = decode(testdata());

    assert_eq!(interface.functions.len(), 7);
    assert_eq!(interface.structs.len(), 2);
    assert_eq!(interface.unions.len(), 1);
    assert_eq!(interface.enums.len(), 1);
    assert_eq!(interface.error_enums.len(), 1);
    assert_eq!(interface.events.len(), 1);
    assert!(!interface.is_empty());
}

#[test]
fn decodes_function_arguments_and_return_types() {
    let (interface, _) = decode(testdata());

    let transfer = function(&interface, "transfer");
    let args: Vec<(&str, &str)> = transfer
        .inputs
        .iter()
        .map(|p| (p.name.as_str(), p.ty.as_str()))
        .collect();
    assert_eq!(
        args,
        [("from", "Address"), ("to", "Address"), ("amount", "i128")]
    );
    assert_eq!(transfer.outputs, ["Result<(), Error>"]);

    // A function returning nothing records no output type at all.
    assert!(function(&interface, "initialize").outputs.is_empty());
}

#[test]
fn renders_nested_and_exotic_types() {
    let (interface, _) = decode(testdata());

    let nested = function(&interface, "nested");
    assert_eq!(nested.inputs[0].ty, "Option<Vec<Map<Symbol, i128>>>");
    assert_eq!(nested.outputs, ["Option<Vec<Map<Symbol, i128>>>"]);

    let lookup = function(&interface, "lookup");
    assert_eq!(lookup.inputs[0].ty, "BytesN<32>");
    assert_eq!(lookup.outputs, ["(u32, Address)"]);

    let scalars = function(&interface, "scalars");
    let types: Vec<&str> = scalars.inputs.iter().map(|p| p.ty.as_str()).collect();
    assert_eq!(types, ["Timepoint", "Duration", "Bytes", "Level"]);
}

#[test]
fn decodes_user_defined_types() {
    let (interface, _) = decode(testdata());

    let config = interface
        .structs
        .iter()
        .find(|s| s.name == "Config")
        .expect("Config struct");
    let fields: Vec<(&str, &str)> = config
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.ty.as_str()))
        .collect();
    assert_eq!(
        fields,
        [
            ("admin", "Address"),
            ("fee_bps", "u32"),
            ("label", "String")
        ]
    );

    // Tuple structs get positional field names.
    let pair = interface
        .structs
        .iter()
        .find(|s| s.name == "Pair")
        .expect("Pair struct");
    assert_eq!(pair.fields[0].name, "0");
    assert_eq!(pair.fields[1].ty, "i128");

    // A union carries void cases and tuple cases side by side.
    let data_key = &interface.unions[0];
    assert_eq!(data_key.name, "DataKey");
    let admin = data_key.cases.iter().find(|c| c.name == "Admin").unwrap();
    assert!(admin.types.is_empty(), "Admin is a void case");
    let allowance = data_key
        .cases
        .iter()
        .find(|c| c.name == "Allowance")
        .unwrap();
    assert_eq!(allowance.types, ["Address", "Address"]);

    let level = &interface.enums[0];
    assert_eq!(level.name, "Level");
    let cases: Vec<(&str, u32)> = level
        .cases
        .iter()
        .map(|c| (c.name.as_str(), c.value))
        .collect();
    assert_eq!(cases, [("Low", 0), ("Medium", 1), ("High", 2)]);

    let errors = &interface.error_enums[0];
    assert_eq!(errors.name, "Error");
    assert_eq!(errors.cases[0].name, "NotAuthorized");
    assert_eq!(errors.cases[0].value, 1);
}

#[test]
fn decodes_events_including_topic_placement() {
    let (interface, _) = decode(testdata());
    let event = &interface.events[0];

    assert_eq!(event.name, "Transfer");
    assert_eq!(event.prefix_topics, ["transfer"]);
    assert_eq!(event.data_format, "map");

    let from = event.params.iter().find(|p| p.name == "from").unwrap();
    assert_eq!(from.location, "topic_list");
    assert_eq!(from.ty, "Address");

    let amount = event.params.iter().find(|p| p.name == "amount").unwrap();
    assert_eq!(amount.location, "data");
}

#[test]
fn preserves_doc_comments() {
    let (interface, _) = decode(testdata());

    assert!(function(&interface, "transfer")
        .doc
        .contains("Returns a result"));

    let config = interface
        .structs
        .iter()
        .find(|s| s.name == "Config")
        .unwrap();
    let admin = config.fields.iter().find(|f| f.name == "admin").unwrap();
    assert!(admin.doc.contains("administer"));
}

#[test]
fn decodes_contract_and_environment_metadata() {
    let (_, meta) = decode(testdata());

    let value = |key: &str| {
        meta.entries
            .iter()
            .find(|e| e.key == key)
            .map(|e| e.value.as_str())
    };
    assert_eq!(value("name"), Some("lumenscan-testdata"));
    assert_eq!(value("version"), Some("0.1.0"));
    // The SDK stamps its own version in as well.
    assert!(value("rssdkver").is_some_and(|v| v.starts_with("27.")));

    let env = meta
        .env_interface_version
        .expect("env interface version is always present");
    assert!(env.protocol >= 20, "unexpected protocol {}", env.protocol);
    assert!(!meta.is_empty());
}

#[test]
fn decodes_a_contract_built_by_an_older_sdk() {
    // The oracle fixture was compiled with SDK 21 and is a real deployed
    // contract, which guards against regressions for older spec encodings.
    let (interface, meta) = decode(oracle());

    let get_value = function(&interface, "get_value");
    assert_eq!(get_value.inputs[0].ty, "String");
    assert_eq!(get_value.outputs, ["OracleValue"]);

    assert!(interface.events.is_empty(), "SDK 21 emitted no event specs");
    assert!(meta
        .entries
        .iter()
        .any(|e| e.key == "rssdkver" && e.value.starts_with("21.")));
}

#[test]
fn rejects_a_wasm_without_a_spec_section() {
    // The smallest valid module: header only, no custom sections.
    let empty_module = b"\0asm\x01\0\0\0";
    let source = Source::Wasm {
        path: "empty.wasm".into(),
    };

    let error = Inspection::from_wasm(source, empty_module).unwrap_err();
    assert!(matches!(error, lumenscan::Error::MissingSpecSection));
    assert!(error.to_string().contains("contractspecv0"));
}

#[test]
fn meta_can_be_read_without_a_spec_section() {
    // `lumenscan meta` should still work where a full inspection cannot.
    let empty_module = b"\0asm\x01\0\0\0";
    let source = Source::Wasm {
        path: "empty.wasm".into(),
    };

    let (_, meta) = Inspection::meta_from_wasm(source, empty_module).unwrap();
    assert!(meta.is_empty());
}
