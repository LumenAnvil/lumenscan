<!-- Thanks for contributing to Lumenscan. -->

## What this changes

<!-- A short description, and the issue it closes if any (Closes #123). -->

## Checklist

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo test` passes (no test requires a network — use a `WasmFetcher` mock)
- [ ] Docs updated if behaviour changed; JSON shape changes bump `SCHEMA_VERSION`
- [ ] `CHANGELOG.md` updated under *Unreleased*
