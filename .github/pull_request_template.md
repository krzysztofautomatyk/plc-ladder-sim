<!--
  This template helps reviewers — and the Copilot cloud agent — verify changes.
  Keep the checklist honest; CI enforces the quality gates.
-->

## Summary

<!-- What does this change do, and why? -->

## Changes

-

## Verification

Confirm the quality gates pass (mirror CI):

- [ ] Frontend: `npm run check` (0 errors / 0 warnings), `npm run test`, `npm run build`
- [ ] Rust (`src-tauri/`): `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`
- [ ] Execution-semantics changes have a matching Rust test (`plc/engine.rs` / `plc/compiler.rs`)
- [ ] No regression to safety/security defaults (no-panic scan, Modbus localhost + write gate, minimal capabilities, audit chain)

## Notes

<!-- Anything reviewers should know: trade-offs, follow-ups, screenshots. -->
