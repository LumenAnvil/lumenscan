# Contributing to Lumenscan

Thanks for taking a look. Lumenscan is a small, read-only tool, and the goal is to keep it that
way: easy to read end to end, easy to extend at a few well-marked seams.

## Getting set up

```console
git clone https://github.com/LumenAnvil/lumenscan
cd lumenscan
make check      # fmt, clippy, tests
```

You need Rust 1.85 or newer. Nothing else — `cargo test` runs entirely offline against the
committed fixtures.

Before opening a pull request:

```console
make check
```

That runs `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test`. All three must be clean.

## The shape of the code

The pipeline is deliberately linear, and each stage is independently testable:

```
contract ID ──► rpc.rs ──► WASM bytes ──► wasm.rs ──► custom sections ──► spec.rs ──► Interface
                                 ▲                                                       │
                          --wasm reads a file                                            ▼
                                                                                     output.rs
```

| Module | Responsibility |
| --- | --- |
| [`src/rpc.rs`](src/rpc.rs) | Fetching WASM, behind the `WasmFetcher` trait |
| [`src/wasm.rs`](src/wasm.rs) | Finding the Soroban custom sections in a module |
| [`src/spec.rs`](src/spec.rs) | Decoding XDR into the `Interface` / `Meta` model |
| [`src/output.rs`](src/output.rs) | Rendering, behind the `Renderer` trait |
| [`src/error.rs`](src/error.rs) | One error enum for the whole crate |
| [`src/main.rs`](src/main.rs) | Argument parsing and dispatch, and nothing else |

## Extension points

### Adding an output format

Implement [`Renderer`](src/output.rs), add a variant to the `Format` enum in
[`src/main.rs`](src/main.rs), and extend the `match` in `Options::renderer`. Nothing else needs
to change — the [`Markdown`](src/output.rs) renderer is a worked example to copy from:

```rust
pub struct Yaml;

impl Renderer for Yaml {
    fn interface(&self, interface: &Interface, meta: &Meta, source: &Source) -> Result<String> {
        // ...
    }

    fn meta(&self, meta: &Meta, source: &Source) -> Result<String> {
        // ...
    }
}
```

### Adding a source of WASM

Implement [`WasmFetcher`](src/rpc.rs) — for a local cache, an archive node, or anything else
that can produce bytes for a contract ID:

```rust
impl WasmFetcher for MyCache {
    fn fetch_wasm(&self, contract: &Contract) -> Result<Vec<u8>> {
        // ...
    }
}
```

This trait is also how the decoder is tested without a network; see
[`tests/rpc_mock.rs`](tests/rpc_mock.rs).

### Changing the JSON output

The `--json` document is a public contract that other tools depend on, and it is documented in
the [README](README.md#json-output). If you change its shape in a way that could break a
consumer, bump `SCHEMA_VERSION` in [`src/lib.rs`](src/lib.rs) and update both the README and
[`tests/json_output.rs`](tests/json_output.rs).

Additive changes (a new key that consumers can ignore) do not need a version bump.

### Supporting new XDR

`spec::type_name` matches exhaustively on `ScSpecTypeDef` with no catch-all arm, and
`decode_interface` does the same on `ScSpecEntry`. That is intentional: when a new variant lands
in a `stellar-xdr` upgrade, these fail to compile rather than silently printing something
misleading. Handle the new variant in both places, and add a case to the fixture contract.

## Tests

- Unit tests live beside the code they test, in a `mod tests` block.
- Integration tests are in [`tests/`](tests/) and run against committed WASM fixtures.
- `cargo test` must never require a network connection. If you need contract bytes, add a
  fixture.

### Working with fixtures

[`tests/fixtures/README.md`](tests/fixtures/README.md) records where each fixture came from.

`testdata.wasm` is built from [`fixtures/testdata-contract/`](fixtures/testdata-contract), a real
Soroban contract in this repository that declares one of every spec entry kind. If you need to
cover a new type or entry kind, add it there and rebuild:

```console
rustup target add wasm32v1-none   # once
make fixtures
```

Commit the regenerated `.wasm` alongside your change.

To capture a fixture from a deployed contract instead:

```console
cargo run --example fetch_fixture -- <CONTRACT_ID> tests/fixtures/name.wasm
```

Record its contract ID, network, and retrieval date in the fixtures README, and prefer small
contracts — these files live in git forever.

## Style

- Match the surrounding code. Small functions, clear names, no cleverness for its own sake.
- Comment the *why*, not the *what*. Most of the existing comments explain a Stellar or XDR
  detail that is not obvious from the code.
- Error messages are user-facing: say what went wrong and what the reader can do about it.
  `Error::StellarAssetContract` is the model — it explains that the contract has no WASM at all
  rather than reporting a decode failure.

## Scope

Lumenscan inspects contracts. It does not deploy them, invoke them, sign anything, or write to
the network, and it has no web UI. Changes in those directions belong in a different tool.
