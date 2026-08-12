# Test fixtures

WASM modules committed so that `cargo test` runs entirely offline.

## `testdata.wasm`

Built from [`fixtures/testdata-contract/`](../../fixtures/testdata-contract), which is part of
this repository and Apache-2.0 licensed. It declares at least one of every `SCSpecEntry` kind
(function, struct, union, enum, error enum, event) plus the awkward corners of the type grammar,
so the decoder is tested against genuine SDK-produced XDR.

Rebuild it after changing the contract:

```console
make fixtures
```

| | |
| --- | --- |
| Source | `fixtures/testdata-contract/` |
| Built with | soroban-sdk 27.0.5, rustc 1.97.1, target `wasm32v1-none` |
| License | Apache-2.0 (this repository) |

## `oracle.wasm`

A real contract deployed to Stellar testnet, kept because it was compiled with a much older SDK
and therefore guards against regressions in decoding older spec encodings (it predates event
specs entirely). Public on-chain bytecode, retrieved from the ledger.

| | |
| --- | --- |
| Contract | `CBTBKYT3OMI4LOUX3UGOZTQSGFAABHHNVRVVGI4SSX3WWNHAIGQAE6TC` |
| Network | Testnet (`https://soroban-testnet.stellar.org`) |
| Retrieved | 2026-08-12 |
| Built with | soroban-sdk 21.6.0 (per its `contractmetav0`) |

Re-download it with:

```console
cargo run --example fetch_fixture -- \
  CBTBKYT3OMI4LOUX3UGOZTQSGFAABHHNVRVVGI4SSX3WWNHAIGQAE6TC \
  tests/fixtures/oracle.wasm
```
