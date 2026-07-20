# Copilot instructions — PLC Ladder Simulator Pro

Desktop **PLC Ladder Diagram (LAD) editor + real-time simulator** with a built-in
**Modbus TCP slave**. Training / lab / integration tool — **not** a certified safety PLC.

## Tech stack

| Layer | Technology |
|-------|------------|
| Desktop shell | **Tauri v2** |
| Core engine | **Rust** (Tokio scan cycle, bytecode, `#![forbid(unsafe_code)]`) |
| UI | **Svelte 5** (runes) + **TypeScript** |
| Fieldbus | **Modbus TCP** slave (`tokio-modbus`, default `127.0.0.1:5020`) |
| Audit | SHA-256 **hash-chained** trail, restored + verified on startup |

## Repository layout

```
src/                         # Svelte 5 + TS frontend
├── app/                     # App shell + global CSS (app.css)
├── shared/
│   ├── lib/                 # types.ts, api.ts (Tauri IPC + browser mock), demoProgram.ts
│   └── stores/plc.svelte.ts # central reactive store (class + runes)
└── features/ladder/
    ├── components/          # LadderEditor, LadderNetwork, LadderElementHost, dialog, toolbar
    ├── lib/                 # addressFormat, ladderLayout, ladderEdit, shortcuts, memoryRead, strokeColors
    └── elements/<name>/     # one folder per IEC instruction: definition.ts + glyph.svelte + index.ts
src-tauri/src/               # Rust backend
├── lib.rs / main.rs / commands.rs / audit.rs
└── plc/                     # memory, compiler, engine, modbus, modbus_map, symbols
.github/workflows/           # ci.yml, release.yml, copilot-setup-steps.yml
docs/                        # ARCHITECTURE.md, MODBUS.md, SETUP.md, RELEASES.md
```

## Commands

Frontend (repo root):

```bash
npm ci                 # install (lockfile)
npm run dev            # Vite dev server on :1420 with a mock backend (browser-only UI work)
npm run build          # production build → dist/
npm run check          # svelte-check (TS + Svelte) — must be 0 errors / 0 warnings
npm run test           # Vitest (unit tests for pure lib modules)
npm run test:coverage  # Vitest with v8 coverage
```

Backend (run inside `src-tauri/`):

```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings   # warnings are errors
cargo fmt --all -- --check
```

Full desktop app: `npm run tauri:dev` / `npm run tauri:build` (needs Rust + Tauri system deps — see docs/SETUP.md).

## Before you finish a change — quality gates (mirror CI)

- **Frontend:** `npm run build`, `npm run test`, `npm run check` all green.
- **Rust:** `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` all green.
- Keep `svelte-check` at **0 warnings** (not just 0 errors). CI also runs `cargo audit` and `cargo llvm-cov`.

## Conventions & architecture notes

- **Instruction packages:** each IEC instruction is a folder under `features/ladder/elements/` with
  `definition.ts` (factory, labels, help), `glyph.svelte` (pure SVG, no store/click logic), `index.ts`.
  Register it with one line in `elements/_shared/registry.ts`. NO and NC are **separate** packages.
- **Rung model:** `Rung.elements` is `RungNode[]` where a node is a `LadderElement` **or** an inline
  `{ type: "parallel", branches }` group. `Rung.or_branches` is the *leading* OR block (seal-in).
  Pure, unit-tested editing operations live in `features/ladder/lib/ladderEdit.ts` — prefer them over
  ad-hoc array mutation in the store. `ElementType` (registry keys) excludes `"parallel"`.
- **Per-element symbolic labels** (e.g. `BTN_START`, max 10 chars) are stored in
  `program.metadata["lbl:<elementId>"]` — display-only, they round-trip through the backend and do
  **not** affect the compiled bytecode or program hash. Use `plc.labelFor(id)` / `plc.setElementLabel`.
- **Compiler → engine:** the AST compiles to bytecode (`OrBegin`/`OrAlt`/`OrEnd`, `Load*`, `Store*`,
  timers/counters, math/move/compare). The engine `OrBegin` **saves incoming power on a stack** so a
  parallel group is AND-ed with the series that reached it (supports mid-rung parallels + nesting).
- **Safety-conscious Rust (keep it this way):** the scan path **never panics** — user-logic faults
  (divide-by-zero, overflow, out-of-range address) return a typed `ScanError` → FAULT/STOP. Use
  checked arithmetic, bounds-checked `PlcMemory`, `parking_lot` locks. `PlcMemory::snapshot()` takes
  all four process-image locks together for a coherent image. A deterministic fuzz test guards no-panic.
- **Security defaults:** Modbus binds to localhost and writes are **disabled** until explicitly enabled
  (`allow_modbus_write`); minimal Tauri capabilities (`src-tauri/capabilities/default.json`); CSP set in
  `tauri.conf.json`; audit trail is an append-only SHA-256 chain restored & re-verified on startup.
- **Frontend reactivity:** Svelte 5 runes. Use `SvelteSet`/reassignment correctly; the store is a class
  with `$state` fields. `api.ts` falls back to a mock when not running under Tauri (browser dev).

## Gotchas

- Adding a `LadderElement` variant cascades to the TS registry (`Record<ElementType, …>`) and the Rust
  `compile_elements` match — update both, plus a glyph + `definition.ts` if it is user-renderable.
- Coils belong on the output rail only; the store routes them there and blocks them from OR branches.
- Don't change execution semantics without a matching Rust test in `plc/engine.rs` / `plc/compiler.rs`.
