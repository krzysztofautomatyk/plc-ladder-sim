---
mode: agent
description: "Review the current diff for correctness, safety and project conventions."
---

# Review changes

Review the current staged/unstaged diff (or the PR branch) and report only
**high-confidence** issues. Focus on:

- **Scan-path safety (Rust):** any new panic path, unchecked arithmetic, direct slice
  indexing with input data, or `unsafe`. The scan must fault, never panic.
- **Determinism & tests:** execution-semantics changes must have a matching test in
  `plc/engine.rs` / `plc/compiler.rs`. Flag missing coverage.
- **Model integrity:** adding a `LadderElement` variant must update the Rust
  `compile_elements` arm, the TS `LadderElement` union, the registry, and a glyph.
- **Security defaults:** Modbus localhost bind + write gate, minimal Tauri
  capabilities, CSP, audit chain — flag any regression.
- **Frontend:** Svelte 5 runes usage, `svelte-check` cleanliness, coils kept on the
  output rail, labels stored in `program.metadata["lbl:<id>"]`.

Ignore pure style/formatting nits (fmt/clippy/svelte-check enforce those). For each
finding give file:line, why it matters, and a concrete fix. If the diff is clean, say so.
