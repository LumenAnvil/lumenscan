# Lumenscan

Inspect a deployed Soroban contract's interface — one command, no guesswork.

```console
$ lumenscan CBTBKYT3OMI4LOUX3UGOZTQSGFAABHHNVRVVGI4SSX3WWNHAIGQAE6TC
contract CBTBKYT3OMI4LOUX3UGOZTQSGFAABHHNVRVVGI4SSX3WWNHAIGQAE6TC

Functions (6)
  admin() -> Address
  get_value(key: String) -> OracleValue
  set_value(key: String, value: OracleValue)
  initialize(admin: Address)
  change_admin(new_admin: Address)
  set_multiple_values(keys: Vec<String>, values: Vec<OracleValue>)

Structs (1)
  OracleValue
    0: u128
    1: u128

Unions (1)
  DataKey
    Admin
    Value(String)

Meta (2)
  env interface version: protocol 21, pre-release 0
  rsver     1.95.0
  rssdkver  21.6.0#f57d2454e520ae11dc9c8d55db610557a24a556e
```

## Why

Every Soroban contract ships its own interface description inside its WASM: the
`contractspecv0` custom section holds a stream of `SCSpecEntry` XDR describing every exported
function, its argument names and types, its return type, and every user-defined struct, union,
enum, and event. Contracts can also embed metadata in `contractmetav0`.

That information is public and complete, but reading it means knowing how to fetch the WASM off
the network (a two-step `getLedgerEntries` lookup: instance entry → code hash → code entry),
where the custom sections live, and how to decode the XDR inside them.

Lumenscan does all of that in one command, and prints the result for people (`lumenscan …`) or
for other tools (`lumenscan … --json`).

It is strictly read-only. It does not deploy, invoke, or modify anything.

## Install

```console
cargo install --path .
```

Or build from source:

```console
git clone https://github.com/LumenAnvil/lumenscan
cd lumenscan
cargo build --release
# binary at target/release/lumenscan
```

Requires Rust 1.85 or newer.

## Usage

```console
lumenscan <CONTRACT_ID>                 # fetch from the network and print the interface
lumenscan --wasm <PATH>                 # same, from a local .wasm file (offline)
lumenscan meta <CONTRACT_ID>            # print only the contract metadata
lumenscan meta --wasm <PATH>
```

Flags, valid on every command:

| Flag | Default | Meaning |
| --- | --- | --- |
| `--format <FORMAT>` | `human` | Output format: `human`, `json`, or `markdown` |
| `--json` | off | Shorthand for `--format json` (the two cannot be combined) |
| `--rpc-url <URL>` | `https://soroban-testnet.stellar.org` | Stellar RPC endpoint |

### Markdown output

`--format markdown` renders the same information as the default listing, but as a document you
can paste straight into a README, a pull request, or generated API docs — a heading per section,
each function as its full signature in inline code, and fields, variants, event parameters and
metadata as tables:

```console
$ lumenscan --wasm my_contract.wasm --format markdown
# my_contract.wasm

## Functions (2)

### `initialize(admin: Address)`

### `transfer(from: Address, to: Address, amount: i128) -> Result<(), Error>`
...
```

Unlike `--json`, the Markdown layout is a convenience for humans rather than a versioned
contract, so it is free to change between releases.

`--rpc-url` also reads the `LUMENSCAN_RPC_URL` environment variable, so you can point at mainnet
or a private node once per shell:

```console
export LUMENSCAN_RPC_URL=https://mainnet.sorobanrpc.com
lumenscan CA...
```

### Inspecting a local build

No network is involved, which makes this useful in CI to check that a contract's published
interface is what you expect:

```console
lumenscan --wasm target/wasm32v1-none/release/my_contract.wasm
```

## JSON output

`--json` emits a stable document intended for other tools. `schema_version` is bumped only on a
breaking change, so consumers can check it and fail loudly.

```console
$ lumenscan --wasm my_contract.wasm --json
```

```json
{
  "schema_version": 1,
  "source": { "kind": "wasm", "path": "my_contract.wasm" },
  "functions": [
    {
      "name": "transfer",
      "doc": "Move tokens between accounts.",
      "inputs": [
        { "name": "from", "type": "Address" },
        { "name": "to", "type": "Address" },
        { "name": "amount", "type": "i128" }
      ],
      "outputs": ["Result<(), Error>"]
    }
  ],
  "structs": [
    {
      "name": "Config",
      "lib": "",
      "fields": [{ "name": "admin", "type": "Address" }]
    }
  ],
  "unions": [
    {
      "name": "DataKey",
      "cases": [
        { "name": "Admin", "types": [] },
        { "name": "Balance", "types": ["Address"] }
      ]
    }
  ],
  "enums": [{ "name": "Level", "cases": [{ "name": "Low", "value": 0 }] }],
  "error_enums": [
    { "name": "Error", "cases": [{ "name": "NotAuthorized", "value": 1 }] }
  ],
  "events": [
    {
      "name": "Transfer",
      "prefix_topics": ["transfer"],
      "params": [{ "name": "from", "type": "Address", "location": "topic_list" }],
      "data_format": "map"
    }
  ],
  "meta": {
    "entries": [{ "key": "rssdkver", "value": "27.0.5" }],
    "env_interface_version": { "protocol": 27, "pre_release": 0 }
  }
}
```

Guarantees you can rely on:

- **`source`** is `{"kind": "contract", "id": "C…"}` or `{"kind": "wasm", "path": "…"}`.
- **All six collections are always present**, as arrays, even when empty. No optional keys to
  handle.
- **`doc` and `lib` are omitted when empty** rather than emitted as `""`.
- **Types are rendered as strings** in Rust-like notation: `u32`, `i128`, `Address`, `Symbol`,
  `BytesN<32>`, `Option<T>`, `Vec<T>`, `Map<K, V>`, `Result<T, E>`, `(A, B)`, and the declared
  name for user-defined types. `()` is the unit type.
- **`outputs` is an array** because that is how the XDR models it, but Soroban permits at most
  one return type.
- **Event `location`** is `"topic_list"` or `"data"`; **`data_format`** is `"single_value"`,
  `"vec"`, or `"map"`.

`lumenscan meta --json` emits `schema_version`, `source`, and `meta` only.

## How it works

```
contract ID ──► rpc.rs ──► WASM bytes ──► wasm.rs ──► custom sections ──► spec.rs ──► Interface
                                 ▲                                                       │
                          --wasm reads a file                                            ▼
                                                                                     output.rs
```

| Module | Responsibility |
| --- | --- |
| [`src/rpc.rs`](src/rpc.rs) | `getLedgerEntries` over JSON-RPC, behind the `WasmFetcher` trait |
| [`src/wasm.rs`](src/wasm.rs) | Pulls `contractspecv0`, `contractmetav0`, `contractenvmetav0` out of the module |
| [`src/spec.rs`](src/spec.rs) | Decodes the XDR into the `Interface` / `Meta` model |
| [`src/output.rs`](src/output.rs) | `Renderer` trait, with human and JSON implementations |

Fetching a deployed contract takes two RPC round trips, matching what Stellar's own tooling
does: read the contract instance ledger entry to get the WASM hash, then read the contract code
entry at that hash.

## Verified dependency versions

The Stellar Rust crates move quickly and rename things between majors, so these were each
checked against the published sources rather than assumed:

| Crate | Version | Notes |
| --- | --- | --- |
| [`stellar-xdr`](https://crates.io/crates/stellar-xdr) | 28.0.0 | XDR types and decoding. **No `curr`/`next` features** — as of v28 the types are re-exported at the crate root, so it is `stellar_xdr::ScSpecEntry`. Built with `base64` for `getLedgerEntries` responses. |
| [`wasmparser`](https://crates.io/crates/wasmparser) | 0.256 | Streams the WASM payloads to find the custom sections. |
| [`ureq`](https://crates.io/crates/ureq) | 3.4 | Blocking HTTP, so there is no async runtime in the dependency tree. |
| [`clap`](https://crates.io/crates/clap) | 4.6 | CLI, `derive` + `env` features. |
| [`serde_json`](https://crates.io/crates/serde_json) | 1 | `--json` output. |
| [`thiserror`](https://crates.io/crates/thiserror) | 2.0 | Error types. |

Lumenscan deliberately does **not** depend on
[`stellar-rpc-client`](https://crates.io/crates/stellar-rpc-client): it pins `stellar-xdr` 27
(which would put two incompatible copies of the XDR types in the build), it pulls in tokio and
jsonrpsee for what amounts to two HTTP POSTs, and its `get_remote_wasm` helper is already marked
deprecated in favour of calling `getLedgerEntries` directly — which is what Lumenscan does.

Verified against Stellar testnet RPC 27.1.1 (protocol 27).

## Development

```console
make check      # fmt --check, clippy -D warnings, and tests
make test
make fixtures   # rebuild the WASM test fixture
```

`cargo test` never touches the network: it runs against WASM fixtures committed under
[`tests/fixtures/`](tests/fixtures/). See [CONTRIBUTING.md](CONTRIBUTING.md) for the extension
points.

## Not in scope

Deploying, invoking, or modifying contracts; a web UI; anything that writes to the network.
Lumenscan reads.

## License

Copyright 2026 goodness-cpu. Licensed under Apache-2.0; see [LICENSE](LICENSE).
