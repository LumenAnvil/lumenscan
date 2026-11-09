---
name: Feature request
about: Suggest a capability or improvement
title: ""
labels: enhancement
assignees: ""
---

**The problem**
What are you trying to do that Lumenscan does not let you do today?

**Proposed solution**
If it is a new output format or a new source of WASM, note that these are
extension points — see CONTRIBUTING.md (`Renderer`, `WasmFetcher`).

**Scope**
Lumenscan is read-only: it fetches and decodes, and never deploys, invokes, or
modifies contracts. Requests that keep it read-only are in scope.
