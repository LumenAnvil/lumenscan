// The decoded-interface model and the XDR → view-model mapping.
//
// This mirrors the Rust crate's `spec.rs`: it turns a contract's
// `ScSpecEntry` stream into a plain, render-ready shape, and names types the
// same way the CLI does (`Vec<String>`, `Option<u32>`, `Result<(), Error>`, …)
// so the web output reads identically to `lumenscan`.

import { xdr } from "@stellar/stellar-sdk";

export interface Param {
  name: string;
  doc: string;
  type: string;
}

export interface Func {
  name: string;
  doc: string;
  inputs: Param[];
  outputs: string[];
}

export interface Field {
  name: string;
  doc: string;
  type: string;
}

export interface Struct {
  name: string;
  doc: string;
  fields: Field[];
}

export interface UnionCase {
  name: string;
  doc: string;
  types: string[];
}

export interface Union {
  name: string;
  doc: string;
  cases: UnionCase[];
}

export interface EnumCase {
  name: string;
  doc: string;
  value: number;
}

export interface Enum {
  name: string;
  doc: string;
  cases: EnumCase[];
}

export interface ErrorEnum {
  name: string;
  doc: string;
  cases: EnumCase[];
}

export interface EventParam {
  name: string;
  doc: string;
  type: string;
  location: "topic_list" | "data";
}

export interface Event {
  name: string;
  doc: string;
  prefixTopics: string[];
  dataFormat: "single-value" | "vec" | "map";
  params: EventParam[];
}

export interface Interface {
  functions: Func[];
  structs: Struct[];
  unions: Union[];
  enums: Enum[];
  errorEnums: ErrorEnum[];
  events: Event[];
}

export function emptyInterface(): Interface {
  return {
    functions: [],
    structs: [],
    unions: [],
    enums: [],
    errorEnums: [],
    events: [],
  };
}

export function isEmptyInterface(i: Interface): boolean {
  return (
    i.functions.length === 0 &&
    i.structs.length === 0 &&
    i.unions.length === 0 &&
    i.enums.length === 0 &&
    i.errorEnums.length === 0 &&
    i.events.length === 0
  );
}

/** A Rust-like name for a spec type, matching the `lumenscan` CLI exactly. */
export function typeName(ty: xdr.ScSpecTypeDef): string {
  switch (ty.type) {
    case "scSpecTypeVal":
      return "Val";
    case "scSpecTypeBool":
      return "bool";
    case "scSpecTypeVoid":
      return "()";
    case "scSpecTypeError":
      return "Error";
    case "scSpecTypeU32":
      return "u32";
    case "scSpecTypeI32":
      return "i32";
    case "scSpecTypeU64":
      return "u64";
    case "scSpecTypeI64":
      return "i64";
    case "scSpecTypeTimepoint":
      return "Timepoint";
    case "scSpecTypeDuration":
      return "Duration";
    case "scSpecTypeU128":
      return "u128";
    case "scSpecTypeI128":
      return "i128";
    case "scSpecTypeU256":
      return "u256";
    case "scSpecTypeI256":
      return "i256";
    case "scSpecTypeBytes":
      return "Bytes";
    case "scSpecTypeString":
      return "String";
    case "scSpecTypeSymbol":
      return "Symbol";
    case "scSpecTypeAddress":
      return "Address";
    case "scSpecTypeMuxedAddress":
      return "MuxedAddress";
    case "scSpecTypeOption":
      return `Option<${typeName(ty.option.valueType)}>`;
    case "scSpecTypeResult":
      return `Result<${typeName(ty.result.okType)}, ${typeName(ty.result.errorType)}>`;
    case "scSpecTypeVec":
      return `Vec<${typeName(ty.vec.elementType)}>`;
    case "scSpecTypeMap":
      return `Map<${typeName(ty.map.keyType)}, ${typeName(ty.map.valueType)}>`;
    case "scSpecTypeTuple":
      return `(${ty.tuple.valueTypes.map(typeName).join(", ")})`;
    case "scSpecTypeBytesN":
      return `BytesN<${ty.bytesN.n}>`;
    case "scSpecTypeUdt":
      return ty.udt.name.toString();
    default:
      // Exhaustive above; a future XDR variant surfaces as its raw tag rather
      // than silently misreporting.
      return (ty as xdr.ScSpecTypeDef).type;
  }
}

function doc(value: { toString(): string }): string {
  return value.toString();
}

/** Build the render-ready interface from a contract's spec entries. */
export function buildInterface(entries: xdr.ScSpecEntry[]): Interface {
  const out = emptyInterface();

  for (const entry of entries) {
    switch (entry.type) {
      case "scSpecEntryFunctionV0": {
        const f = entry.functionV0;
        // Soroban reserves `__` for internal functions (e.g. `__constructor`
        // style hooks); the CLI lists user functions only.
        const name = f.name.toString();
        out.functions.push({
          name,
          doc: doc(f.doc),
          inputs: f.inputs.map((input) => ({
            name: input.name.toString(),
            doc: doc(input.doc),
            type: typeName(input.type),
          })),
          outputs: f.outputs.map(typeName),
        });
        break;
      }
      case "scSpecEntryUdtStructV0": {
        const s = entry.udtStructV0;
        out.structs.push({
          name: s.name.toString(),
          doc: doc(s.doc),
          fields: s.fields.map((field) => ({
            name: field.name.toString(),
            doc: doc(field.doc),
            type: typeName(field.type),
          })),
        });
        break;
      }
      case "scSpecEntryUdtUnionV0": {
        const u = entry.udtUnionV0;
        out.unions.push({
          name: u.name.toString(),
          doc: doc(u.doc),
          cases: u.cases.map((c) => {
            if (c.type === "scSpecUdtUnionCaseTupleV0") {
              return {
                name: c.tupleCase.name.toString(),
                doc: doc(c.tupleCase.doc),
                types: c.tupleCase.type.map(typeName),
              };
            }
            return {
              name: c.voidCase.name.toString(),
              doc: doc(c.voidCase.doc),
              types: [],
            };
          }),
        });
        break;
      }
      case "scSpecEntryUdtEnumV0": {
        const e = entry.udtEnumV0;
        out.enums.push({
          name: e.name.toString(),
          doc: doc(e.doc),
          cases: e.cases.map((c) => ({
            name: c.name.toString(),
            doc: doc(c.doc),
            value: c.value,
          })),
        });
        break;
      }
      case "scSpecEntryUdtErrorEnumV0": {
        const e = entry.udtErrorEnumV0;
        out.errorEnums.push({
          name: e.name.toString(),
          doc: doc(e.doc),
          cases: e.cases.map((c) => ({
            name: c.name.toString(),
            doc: doc(c.doc),
            value: c.value,
          })),
        });
        break;
      }
      case "scSpecEntryEventV0": {
        const ev = entry.eventV0;
        out.events.push({
          name: ev.name.toString(),
          doc: doc(ev.doc),
          prefixTopics: ev.prefixTopics.map((t) => t.toString()),
          dataFormat: dataFormatName(ev.dataFormat),
          params: ev.params.map((p) => ({
            name: p.name.toString(),
            doc: doc(p.doc),
            type: typeName(p.type),
            location:
              p.location.name === "scSpecEventParamLocationTopicList"
                ? "topic_list"
                : "data",
          })),
        });
        break;
      }
    }
  }

  return out;
}

function dataFormatName(
  f: xdr.ScSpecEventDataFormat,
): "single-value" | "vec" | "map" {
  switch (f.name) {
    case "scSpecEventDataFormatVec":
      return "vec";
    case "scSpecEventDataFormatMap":
      return "map";
    default:
      return "single-value";
  }
}
