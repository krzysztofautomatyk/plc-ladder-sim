# AGENTS.md

Guidance for AI coding agents (GitHub Copilot, and any tool that reads `AGENTS.md`)
working in this repository. GitHub Copilot also reads
[`.github/copilot-instructions.md`](.github/copilot-instructions.md) and the
path-scoped rules in [`.github/instructions/`](.github/instructions).

## What this is

**PLC Ladder Simulator Pro** — a desktop PLC Ladder Diagram (LAD) editor and
real-time simulator with a built-in Modbus TCP slave.

- **Backend:** Rust + Tauri v2 (`src-tauri/`) — deterministic scan engine, bytecode
  compiler, process image, Modbus slave, SHA-256 audit chain. `#![forbid(unsafe_code)]`.
- **Frontend:** Svelte 5 (runes) + TypeScript (`src/`) — ladder editor, watch table,
  symbols, Modbus config.

> Training / lab tool. **Not** a certified safety PLC — do not remove the safety
> disclaimers or weaken the safety/security posture described below.

## Setup & commands

```bash
npm ci                 # install frontend deps (lockfile)
npm run dev            # Vite dev server on :1420 with a mock backend (UI-only work)
npm run build          # frontend production build
npm run check          # svelte-check — must be 0 errors / 0 warnings
npm run test           # Vitest unit tests
npm run tauri:dev      # full desktop app (needs Rust + Tauri system deps, see docs/SETUP.md)
```

Rust (run inside `src-tauri/`):

```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings   # warnings are errors
cargo fmt --all -- --check
```

## Definition of done (mirror CI before you finish)

- Frontend: `npm run build`, `npm run test`, `npm run check` all green (check at 0/0).
- Rust: `cargo fmt --all -- --check`, `cargo clippy … -D warnings`, `cargo test --all-features` all green.
- CI additionally runs `cargo audit` and `cargo llvm-cov`.

## Rules that matter most

1. **The scan path never panics.** Faults (divide-by-zero, overflow, out-of-range)
   return a typed `ScanError` → FAULT/STOP. Use checked arithmetic and bounds-checked
   `PlcMemory`. Add a Rust test for any execution-semantics change.
2. **Security defaults are intentional:** Modbus binds to localhost and writes are
   disabled until enabled; minimal Tauri capabilities; CSP set; audit chain restored
   and verified on startup. Don't regress these.
3. **Instruction packages:** one folder per IEC instruction under
   `src/features/ladder/elements/`; glyphs are pure SVG (no store/click logic);
   register with one line in `_shared/registry.ts`.
4. **Rung model:** `Rung.elements` is `RungNode[]` (elements + inline `parallel`
   groups); edit via the pure helpers in `features/ladder/lib/ladderEdit.ts`. Coils
   only on the output rail. Per-element labels live in `program.metadata["lbl:<id>"]`.
5. Keep changes surgical and consistent with surrounding code; don't reformat unrelated
   files (fmt/clippy/svelte-check own formatting).

## Reusable prompts

`.github/prompts/` contains ready-made prompt files: `new-ladder-instruction`,
`verify-changes`, `review-changes`.

## Layout

See [`.github/copilot-instructions.md`](.github/copilot-instructions.md) and
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full directory map and design.
