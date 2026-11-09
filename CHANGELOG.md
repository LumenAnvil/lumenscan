# Changelog

All notable changes to Lumenscan are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project aims to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--format markdown`, a Markdown renderer for a decoded interface, behind the
  same `Renderer` trait as the human and JSON output.
- GitHub Actions CI: format, Clippy, tests and docs, plus a build on the
  declared MSRV.
- A companion browser inspector, now in its own repository,
  [LumenAnvil/lumenscan-web](https://github.com/LumenAnvil/lumenscan-web)
  (live at <https://lumenanvil.github.io/lumenscan-web/>), sharing this crate's
  `oracle.wasm` test fixture.

### Changed

- Minimum supported Rust version is **1.85** (a lockfile dependency uses the
  2024 edition).

## [0.1.0]

### Added

- Initial release: fetch a deployed contract's WASM over RPC (or read a local
  `.wasm`), decode its `contractspecv0` interface and `contractmetav0` metadata,
  and render them as a human-readable table or as JSON.
- Exhaustive decoding of `ScSpecEntry` and `ScSpecTypeDef`, so a new XDR variant
  fails the build rather than printing something misleading.
- A stable, versioned JSON document (`SCHEMA_VERSION`) for consumption by other
  tools.

[Unreleased]: https://github.com/LumenAnvil/lumenscan/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/LumenAnvil/lumenscan/releases/tag/v0.1.0
