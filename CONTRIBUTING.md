# Contributing to PLC Ladder Simulator Pro

Thank you for helping improve **PLC Ladder Simulator Pro**. This document keeps the project healthy for both first-time contributors and long-term maintainers.

More environment detail: [docs/SETUP.md](./docs/SETUP.md) · Architecture: [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) · Releases: [docs/RELEASES.md](./docs/RELEASES.md)

## Ground rules

- Be respectful and constructive.
- Prefer small, focused pull requests over large multi-topic changes.
- Match the existing style (Rust 2021, TypeScript / Svelte 5, clear names).
- Do not commit secrets, personal paths, or generated `src-tauri/target/` / `node_modules/` / `dist/` artifacts.
- One IEC ladder instruction = **one folder** under `src/features/ladder/elements/<name>/` (see conventions below).

## Prerequisites (install these first)

`cargo` is **not** a Windows built-in. It comes with the **Rust** toolchain.  
If PowerShell or CMD says `cargo` is not recognized, Rust is missing or the terminal was not restarted after install.

### 1. Git

- Download: https://git-scm.com/  
- During Windows setup, keep “Git from the command line” enabled.

### 2. Node.js 20+ (includes `npm`)

- Download LTS: https://nodejs.org/  
- Verify in a **new** terminal:

  ```bash
  node -v
  npm -v
  ```

  PowerShell:

  ```powershell
  node -v
  npm -v
  ```

### 3. Rust + Cargo (required for the PLC core and Tauri)

**Windows (recommended):**

1. Open https://rustup.rs/ and run `rustup-init.exe`, **or** in PowerShell:

   ```powershell
   winget install Rustlang.Rustup
   ```

2. When the installer asks about the toolchain, choose the default (**stable**).
3. Install the **MSVC** build tools if prompted (Visual Studio Build Tools / “Desktop development with C++”).  
   Tauri on Windows needs the MSVC linker; pure MinGW often fails later.
4. **Close and reopen** the terminal (PATH is updated only for new sessions).
5. Verify:

   ```powershell
   rustc -V
   cargo -V
   ```

You should see version numbers. If not:

```powershell
# Ensure cargo is on PATH (typical location)
$env:Path += ";$env:USERPROFILE\.cargo\bin"
cargo -V
```

Permanent fix: add `%USERPROFILE%\.cargo\bin` to your user PATH in Windows Settings → Environment variables, then open a new terminal.

**macOS / Linux:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# restart shell, then:
rustc -V && cargo -V
```

Rust **1.77+** is required (`rust-version` in `src-tauri/Cargo.toml`).

### 4. Tauri system dependencies

Follow the official list for your OS:  
https://v2.tauri.app/start/prerequisites/

#### Windows

In practice you need:

- **WebView2** (usually already on Windows 10/11; install the Evergreen runtime if missing)
- **Visual Studio Build Tools 2022** with workload **“Desktop development with C++”**  
  (or full Visual Studio with that workload)

#### macOS

```bash
xcode-select --install
```

#### Linux (Debian/Ubuntu)

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  librsvg2-dev \
  patchelf \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev
```

Prefer **Ayatana** only — do not install both classic `libappindicator3-dev` and Ayatana stacks (apt conflict).

### 5. Optional but recommended

- `RUST_LOG` for engine debug (e.g. `RUST_LOG=info,plc_ladder_sim_lib=debug`)

### Quick health check

```bash
git --version
node -v
npm -v
rustc -V
cargo -V
```

PowerShell:

```powershell
git --version
node -v
npm -v
rustc -V
cargo -V
```

All five should print versions before you continue.

---

## Development setup

### Clone and install frontend deps

```bash
git clone https://github.com/krzysztofautomatyk/plc-ladder-sim.git
cd plc-ladder-sim
npm install
```

PowerShell is the same (clone / `cd` / `npm install`).

### Run the app (recommended)

From the **repository root** (not `src-tauri/` alone):

```bash
npm run tauri:dev
```

What should happen:

1. Vite starts the UI (`http://localhost:1420`)
2. Rust compiles the desktop shell + PLC engine
3. The **PLC Ladder Simulator Pro** window opens

First compile can take several minutes; later runs are faster.

### Frontend-only UI iteration (no native Modbus)

Useful for pure Svelte work; Tauri IPC falls back to mocks:

```bash
npm run dev
# open http://localhost:1420
```

### Run checks before opening a PR

```bash
# Frontend
npm run build
npm run check

# Rust core (from repo root)
cd src-tauri
cargo test
cd ..
```

Optional stricter Rust hygiene:

```bash
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets
cd ..
```

### Production build (local)

```bash
npm run tauri:build
```

Artifacts: `src-tauri/target/release/bundle/`  
CI builds installers for Windows / macOS / Linux on version tags — see [docs/RELEASES.md](./docs/RELEASES.md).

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|--------|----------------|-----|
| `'cargo' is not recognized` | Rust not installed or old terminal | Install [rustup](https://rustup.rs/), **restart terminal**, ensure `~/.cargo/bin` (or `%USERPROFILE%\.cargo\bin`) is on PATH |
| `'npm' is not recognized` | Node not installed | Install Node LTS from nodejs.org, restart terminal |
| `link.exe` / MSVC errors | Missing C++ build tools | Install VS Build Tools → “Desktop development with C++” |
| WebView2 errors | Runtime missing | Install [WebView2 Evergreen](https://developer.microsoft.com/microsoft-edge/webview2/) |
| Blank window / port in use | Vite **1420** conflict | Stop other dev servers; check terminal logs |
| Modbus bind failed | Port **5020** in use | Free the port or change default in `src-tauri/src/plc/modbus.rs` |
| `glib-2.0` / webkit pkg-config (Linux) | Missing Tauri apt packages | Install Linux deps from the prerequisites section above |
| Icon / bundle errors on `tauri build` | Incomplete `src-tauri/icons` | Keep the full icon set from `src-tauri/icons/` (see release workflow) |
| Slow first `npm run tauri:dev` | Cold compile | Normal; wait for `Finished` |

### Still stuck?

Open a GitHub issue with:

1. OS and version  
2. Output of `rustc -V`, `cargo -V`, `node -v`, `npm -v`  
3. Full error text from the terminal  

---

## Project layout

| Path | Responsibility |
|------|----------------|
| `src/app/` | Shell UI (TIA-style layout) + global CSS |
| `src/features/ladder/` | LAD editor, palette, element host, layout helpers |
| `src/features/ladder/elements/` | **One folder per IEC instruction** (definition + glyph) |
| `src/features/watch/` | Process image / watch panel |
| `src/features/symbols/` | Symbol table UI |
| `src/features/modbus/` | Modbus map / status UI |
| `src/features/audit/` | Audit trail UI |
| `src/shared/lib/` | Domain types, Tauri IPC, demo program |
| `src/shared/stores/plc.svelte.ts` | App state (Svelte 5 runes) |
| `src-tauri/src/plc/` | Compiler, scan engine, memory, Modbus slave |
| `src-tauri/src/commands.rs` | Tauri command surface |
| `src-tauri/src/audit.rs` | Hash-chained audit trail |
| `docs/` | SETUP, ARCHITECTURE, MODBUS, RELEASES |

### Ladder instruction packages

Every instruction lives in its own package:

```text
src/features/ladder/elements/<name>/
  definition.ts   # kind, labels, help, create()
  glyph.svelte    # pure graphic (SVG / FB box) — no store / clicks
  index.ts        # export { definition, Glyph }
```

Register the package in `src/features/ladder/elements/_shared/registry.ts` (one entry).

Rules:

1. **NO and NC are separate packages** — never a shared component with a `negated` flag.
2. Glyphs are pure presentation (strokes / box only).
3. Factory + labels + help live in `definition.ts`, not in the store or host.
4. Domain AST types stay in `src/shared/lib/types.ts`.

Keep pure PLC logic in Rust (`src-tauri/src/plc/`) so it stays unit-testable without the GUI.

## Pull request checklist

- [ ] Tests added or updated for engine / compiler behavior you change (`cd src-tauri && cargo test`)
- [ ] Frontend builds (`npm run build`)
- [ ] New ladder instruction (if any) has its own `elements/<name>/` folder + registry line
- [ ] No unrelated formatting churn
- [ ] No `node_modules/`, `target/`, secrets, or personal absolute paths

## Commit messages

Use clear, imperative subjects, for example:

- `fix(engine): fault on divide-by-zero instead of panic`
- `feat(ladder): add contact rising-edge glyph package`
- `docs: clarify Windows install steps for cargo`
- `fix(release): regenerate Tauri icons for multi-platform bundle`

## Reporting bugs

Open a GitHub issue with:

1. OS and app version / commit / release tag
2. Steps to reproduce
3. Expected vs actual behavior
4. Ladder program JSON snippet if relevant (redact secrets)
5. Whether you used a **prebuilt installer** or built from source

## Releases

Prebuilt installers (Windows / macOS / Linux) are produced by GitHub Actions on tags `v*`.  
Maintainers: see [docs/RELEASES.md](./docs/RELEASES.md).

## Security

Do **not** file public issues for vulnerabilities. See [SECURITY.md](./SECURITY.md).

This application is a **lab / training / integration simulator**, not a certified safety PLC. See the disclaimer in [README.md](./README.md) and [SECURITY.md](./SECURITY.md).

## Code of conduct

By participating you agree to follow our [Code of Conduct](./CODE_OF_CONDUCT.md).

## License

By contributing, you agree that your contributions are licensed under the **MIT License**, the same as the project ([LICENSE](./LICENSE)).
