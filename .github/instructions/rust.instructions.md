---
applyTo: "src-tauri/**/*.rs"
---

# Rust / Tauri backend instructions

This is the **safety-conscious** core of a PLC simulator. Correctness and
determinism matter more than cleverness.

## Non-negotiable rules

- **Never panic in the scan path.** User-logic faults (divide-by-zero, overflow,
  out-of-range address) must return a typed `ScanError` → the engine sets
  FAULT/STOP. Use checked arithmetic (`checked_add`, `checked_mul`, …) and the
  bounds-checked `PlcMemory` accessors — never index slices directly with input data.
- `#![forbid(unsafe_code)]` is set in `lib.rs`/`main.rs`. Do not introduce `unsafe`.
- Locks are `parking_lot`. `PlcMemory::snapshot()` takes all four process-image
  locks together for a coherent image — keep it that way.
- Modbus stays **localhost-bound** and **write-gated** (`allow_modbus_write`).
  Don't widen the bind address or remove the write gate.
- The audit trail is an append-only SHA-256 chain restored + verified on startup.
  Don't break `verify_chain`; new audited actions call `AuditTrail::record`.

## Conventions

- Errors use `thiserror` enums; commands return `CommandResult<T>` (`ok`/`err`).
- Serde enums are `#[serde(tag = "type", rename_all = "snake_case")]`. When adding a
  `LadderElement` variant, also add the arm in `compiler::compile_elements` and, if it
  is renderable, a TS type + registry entry + glyph on the frontend.
- The compiled program **hash** is derived from bytecode only — display-only fields
  (labels, comments) must not change it.

## Definition of done (run inside `src-tauri/`)

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings   # warnings are errors
cargo test --all-features
```

Any change to execution semantics needs a matching test in `plc/engine.rs` or
`plc/compiler.rs`. Prefer a deterministic test (see the existing LCG-based fuzz
tests) over randomness with a real RNG.
