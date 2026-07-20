# Architecture

## Stack

| Layer | Technology |
|--------|------------|
| Desktop shell | Tauri v2 |
| Core engine | Rust (Tokio scan cycle, bytecode) |
| UI | Svelte 5 + TypeScript |
| Fieldbus | Modbus TCP slave (`tokio-modbus`) |
| Audit | SHA-256 hash-chained trail |

## Runtime overview

```
┌─────────────────────────────────────────────────────────┐
│  Svelte 5 UI  (ladder editor · watch · symbols · …)     │
│       │ invoke / events                                  │
│       ▼                                                  │
│  Tauri Commands  (update_program, start/stop, …)        │
│       │                                                  │
│  ┌────┴─────┐    ┌──────────────┐    ┌───────────────┐  │
│  │ Compiler │───▶│ Scan Engine  │───▶│  PlcMemory    │  │
│  │ AST→BC   │    │ Tokio interval│    │ Arc + locks   │  │
│  └──────────┘    └──────────────┘    └───────┬───────┘  │
│                                              │          │
│                                    ┌─────────▼────────┐ │
│                                    │ Modbus TCP :5020 │ │
│                                    └──────────────────┘ │
│  AuditTrail (hash chain) ← operator / system actions    │
└─────────────────────────────────────────────────────────┘
```

## Frontend layout

```
src/
├── main.ts
├── app/                    # shell + global CSS
├── shared/
│   ├── lib/                # types, api, demo program
│   └── stores/             # plc.svelte.ts
└── features/
    ├── ladder/
    │   ├── components/     # editor chrome
    │   ├── lib/            # address, layout, strokes
    │   └── elements/       # ★ one folder per IEC instruction
    ├── watch/
    ├── symbols/
    ├── modbus/
    ├── audit/
    └── docs/
```

### Instruction packages

Each ladder instruction is a package:

```
features/ladder/elements/<name>/
  definition.ts   # kind, labels, help, create()
  glyph.svelte    # pure graphic
  index.ts
```

Registered in `elements/_shared/registry.ts`.

Rules:

1. One IEC instruction = one folder  
2. NO and NC are separate packages  
3. Glyphs have no store / click logic  
4. Factory + labels live in `definition.ts`  

## Backend layout

```
src-tauri/src/
├── main.rs / lib.rs
├── commands.rs
├── audit.rs
└── plc/
    ├── memory.rs
    ├── compiler.rs
    ├── engine.rs
    ├── modbus.rs
    ├── modbus_map.rs
    └── symbols.rs
```

## Tauri commands (selected)

| Command | Purpose |
|---------|---------|
| `update_program` | Compile ladder JSON → bytecode |
| `start_simulation` / `stop_simulation` | Scan control |
| `get_memory_snapshot` | Process image |
| `set_cycle_ms` | 5–100 ms scan period |
| `set_discrete_input` | Force I bits from UI |
| `export_program_json` / `import_program_json` | Program exchange |
| `export_audit_report` / `verify_audit_chain` | Audit |

## Events (backend → UI)

| Event | Payload |
|-------|---------|
| `plc://scan-tick` | Active elements/rungs, metrics |
| `plc://memory` | Memory snapshot |
| `plc://fault` | Fault stop |

## Scan semantics

The engine follows the IEC 61131-3 cyclic model with one **documented, deliberate**
refinement:

- **Discrete inputs (I) and input registers (IW)** are captured **once** into a
  frozen process-image at the start of every scan (`ScanInputImage::capture`).
  All contact reads within the scan see this stable snapshot.
- **Coils (Q) and holding markers (M/R)** are read **live** during rung
  evaluation. This is intentional: it lets an output referenced later in the same
  scan (e.g. an OR **seal-in** of `Q0`) reflect the value just written, which is
  the behaviour operators expect from a teaching simulator. It is a conscious
  divergence from controllers that snapshot the *entire* image; it is safe here
  because outputs are single-writer within a scan.
- `PlcMemory::snapshot()` takes all four process-image locks together, so the
  image exposed to the UI / Modbus is **coherent** across areas.

## Quality notes

- User-logic faults (e.g. divide-by-zero, arithmetic overflow, out-of-range
  address) → **FAULT / STOP** with a typed `ScanError` — **never a panic** in the
  scan path (verified by a deterministic fuzz sweep in the engine tests).
- Program integrity: SHA-256 of compiled bytecode.
- **Audit trail spans restarts:** the SHA-256 hash chain is mirrored to an
  append-only `audit_trail.jsonl` and **restored + re-verified on startup**
  (`AuditTrail::load_persisted`), so tamper-evidence is not reset each session.
- Release profile: LTO, `panic = "abort"`, stripped symbols.

**Not certified** for safety-critical or clinical use — see [SECURITY.md](../SECURITY.md).
