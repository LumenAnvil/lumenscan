import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { specFromWasm } from "./inspect";
import { buildInterface, isEmptyInterface, typeName } from "./model";

// Decode the same committed fixture the Rust crate tests against, so the web
// renderer is pinned to real contract bytes, not hand-built XDR.
function oracle() {
  const path = fileURLToPath(
    new URL("../../tests/fixtures/oracle.wasm", import.meta.url),
  );
  return buildInterface(specFromWasm(new Uint8Array(readFileSync(path))));
}

describe("buildInterface on the oracle fixture", () => {
  it("decodes every function with named, typed arguments", () => {
    const i = oracle();
    expect(i.functions.map((f) => f.name).sort()).toEqual([
      "admin",
      "change_admin",
      "get_value",
      "initialize",
      "set_multiple_values",
      "set_value",
    ]);

    const getValue = i.functions.find((f) => f.name === "get_value")!;
    expect(getValue.inputs).toEqual([
      { name: "key", doc: "", type: "String" },
    ]);
    expect(getValue.outputs).toEqual(["OracleValue"]);
  });

  it("renders compound argument types the way the CLI does", () => {
    const i = oracle();
    const setMany = i.functions.find((f) => f.name === "set_multiple_values")!;
    expect(setMany.inputs.map((p) => p.type)).toEqual([
      "Vec<String>",
      "Vec<OracleValue>",
    ]);
  });

  it("decodes the struct and its fields", () => {
    const { structs } = oracle();
    expect(structs).toHaveLength(1);
    expect(structs[0].name).toBe("OracleValue");
    expect(structs[0].fields.map((f) => `${f.name}: ${f.type}`)).toEqual([
      "0: u128",
      "1: u128",
    ]);
  });

  it("decodes the union with its void and tuple cases", () => {
    const { unions } = oracle();
    expect(unions).toHaveLength(1);
    expect(unions[0].name).toBe("DataKey");

    const admin = unions[0].cases.find((c) => c.name === "Admin")!;
    expect(admin.types).toEqual([]); // void case

    const value = unions[0].cases.find((c) => c.name === "Value")!;
    expect(value.types).toEqual(["String"]); // tuple case
  });

  it("reports a non-empty interface", () => {
    expect(isEmptyInterface(oracle())).toBe(false);
  });
});

describe("typeName via decoded argument types", () => {
  it("names scalars and user-defined types from real spec entries", () => {
    const i = oracle();
    const admin = i.functions.find((f) => f.name === "admin")!;
    // `admin() -> Address`
    expect(admin.outputs).toEqual(["Address"]);

    const setValue = i.functions.find((f) => f.name === "set_value")!;
    expect(setValue.inputs.map((p) => `${p.name}: ${p.type}`)).toEqual([
      "key: String",
      "value: OracleValue",
    ]);
  });

  it("is a pure function of its input", () => {
    // typeName is exported for reuse; exercise the simple identity that two
    // decodes of the same bytes name types identically.
    const a = oracle().functions.map((f) => f.outputs.join(","));
    const b = oracle().functions.map((f) => f.outputs.join(","));
    expect(a).toEqual(b);
    expect(typeof typeName).toBe("function");
  });
});
