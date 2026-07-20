# PLC Ladder Simulator Pro

**Desktop PLC Ladder Diagram (LAD) editor and real-time simulator** with a built-in **Modbus TCP slave** for SCADA / lab integration.

[![CI](https://github.com/krzysztofautomatyk/plc-ladder-sim/actions/workflows/ci.yml/badge.svg)](https://github.com/krzysztofautomatyk/plc-ladder-sim/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/krzysztofautomatyk/plc-ladder-sim?include_prereleases&label=release)](https://github.com/krzysztofautomatyk/plc-ladder-sim/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri)](https://tauri.app)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte)](https://svelte.dev)
[![Rust](https://img.shields.io/badge/Rust-1.77+-orange?logo=rust)](https://www.rust-lang.org)

| Layer | Stack |
|--------|--------|
| Desktop shell | **Tauri v2** |
| Core engine | **Rust** (Tokio scan cycle, bytecode) |
| UI | **Svelte 5** + TypeScript |
| Fieldbus | **Modbus TCP** slave (default port **5020**) |
| Audit | SHA-256 **hash-chained** audit trail |

> **Disclaimer:** Training / lab / integration tool. Practices inspired by high-assurance systems (fault → STOP, program hash, audit chain, minimal Tauri capabilities). **Not certified** as a safety PLC or medical device. See [SECURITY.md](./SECURITY.md).

---

## Install

### Prebuilt desktop apps (recommended)

Download the installer for your OS from the latest GitHub Release (same pattern as [ProjectToText](https://github.com/krzysztofautomatyk/ProjectToText/releases)):

**→ [Releases](https://github.com/krzysztofautomatyk/plc-ladder-sim/releases/latest)**

| Platform | Typical assets |
|----------|----------------|
| **Windows** | `.msi` / `.exe` (NSIS) |
| **macOS** | `.dmg` (Apple Silicon + Intel) |
| **Linux** | `.deb` / `.AppImage` |

After install, launch **PLC Ladder Simulator Pro** from Applications / Start Menu.

v1 builds are **unsigned** (expect Gatekeeper / SmartScreen prompts). Details: [docs/RELEASES.md](./docs/RELEASES.md).

---

## Features

- **Graphical LAD editor** — rungs, drag-and-drop, multi-coil stack, address dialog (I/Q/M/R, `R1.x`)
- **Bit logic** — NO / NC contacts, rising / falling edge, coil, negated coil, **SET / RESET**
- **Parallel OR branches** on rungs
- **Timers / counters** — TON, TOF, RTO, CTU, CTD
- **Math / MOVE / Compare**
- **Real-time scan** — cycle **5–100 ms**, live power-flow highlight
- **Modbus TCP slave** — coils, discrete, holding & input registers
- **Symbol table** + Modbus map UI
- **Export / import** — program JSON + bytecode
- **Audit trail** — append-only SHA-256 hash chain, **restored & re-verified on startup**, with report export

---

## Quick start (from source)

```bash
git clone https://github.com/krzysztofautomatyk/plc-ladder-sim.git
cd plc-ladder-sim
npm install
npm run tauri:dev
```

Prerequisites and troubleshooting: **[docs/SETUP.md](./docs/SETUP.md)**.

Frontend-only UI work: `npm run dev` → http://localhost:1420 (mock backend).

---

## Architecture

```
UI (Svelte 5) ──invoke/events──▶ Tauri commands
                                      │
                    compiler (AST→bytecode) → PlcEngine (scan)
                                      │
                                 PlcMemory (shared process image)
                                      │
                              Modbus TCP :5020
```

- Full layout: **[docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)**
- Modbus map & diagnostics: **[docs/MODBUS.md](./docs/MODBUS.md)**
- Installers & tags: **[docs/RELEASES.md](./docs/RELEASES.md)**

### Instruction packages (frontend)

Each IEC instruction lives in its own folder:

```
src/features/ladder/elements/contact-no/
  definition.ts   # factory, labels, help
  glyph.svelte    # pure SVG graphic
  index.ts
```

NO and NC are **separate** packages. New instruction = new folder + one registry line.

---

## Demo program

On boot the engine loads **Demo_Start_Stop**:

1. **(I0 OR Q0) AND NOT I1 → Q0** — seal-in via OR branches  
2. **Q0 → TON 2000 ms → Q1**  
3. **I2** edges → **CTU** → **Q2** (reset **I3**)  
4. Compare / MOVE demo rungs as shipped  

Toggle inputs in the **Process Image** panel while RUN is active.

---

## Development

```bash
npm run build                 # frontend production build
cd src-tauri && cargo test    # engine / compiler tests
```

Contributing guide: [CONTRIBUTING.md](./CONTRIBUTING.md).

CI runs on every push/PR. Multi-platform installers build on tags `v*` (e.g. `v1.0.0`).

---

## License

[MIT](./LICENSE) © PLC Ladder Simulator contributors
