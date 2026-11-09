# Security policy

Lumenscan is a read-only inspector: it fetches a contract's WASM and decodes it,
and never deploys, invokes, signs, or writes to the network. It handles no keys
and holds no secrets.

## Reporting a vulnerability

Please report security issues privately through GitHub Security Advisories
rather than as a public issue:

<https://github.com/LumenAnvil/lumenscan/security/advisories/new>

Things worth reporting:

- a crafted contract spec or `.wasm` that causes Lumenscan to crash, hang, or
  consume unbounded memory while decoding,
- any path by which inspecting a contract could cause a write to the network
  (it must never do this),
- a supply-chain concern in the dependencies.

You can expect an acknowledgement, and we will keep you updated as we work on a
fix.
