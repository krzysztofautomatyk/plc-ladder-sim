# Setup — PLC Ladder Simulator Pro

Development and SCADA lab setup guide.

## 1. Prerequisites

| Tool | Version | Notes |
|------|---------|--------|
| **Rust** | 1.77+ | [rustup.rs](https://rustup.rs) |
| **Node.js** | 20+ LTS | npm included |
| **Platform deps** | OS-specific | [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) |

### macOS

```bash
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install node   # example
```

### Linux (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Windows

- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (C++ workload)
- [Rust](https://rustup.rs) and [Node.js](https://nodejs.org)
- WebView2 (usually preinstalled on Windows 10/11)

## 2. Install & run

```bash
git clone https://github.com/YOUR_GITHUB_USER/plc-ladder-sim.git
cd plc-ladder-sim
npm install
npm run tauri:dev
```

First run compiles the Rust core (several minutes). Later runs are incremental.

### Production build (local)

```bash
npm run tauri:build
```

Artifacts: `src-tauri/target/release/bundle/`  
Prebuilt installers for all platforms: see [RELEASES.md](./RELEASES.md).

### Frontend only (UI iteration)

Backend calls fall back to mocks when not running under Tauri:

```bash
npm run dev
# http://localhost:1420
```

## 3. Environment variables

| Variable | Purpose |
|----------|---------|
| `RUST_LOG` | e.g. `info,plc_ladder_sim_lib=debug` |
| `TAURI_DEV_HOST` | Vite HMR host (optional) |

```bash
RUST_LOG=debug npm run tauri:dev
```

## 4. Demo workflow

1. Launch app → demo program loads
2. Click **▶ RUN**
3. Toggle **I0** (start) → **Q0** seal-in
4. Toggle **I1** (stop) → **Q0** drops
5. With **Q0** on, wait ~2 s → **Q1** (TON)
6. Pulse **I2** five times → **Q2** (CTU); **I3** resets
7. Optional: connect SCADA to `:5020` — see [MODBUS.md](./MODBUS.md)

## 5. Tests & checks

```bash
cd src-tauri && cargo test && cd ..
npm run build
npm run check   # Svelte/TS check
```

## 6. Audit log location

| OS | Typical path |
|----|----------------|
| macOS | `~/Library/Application Support/com.plc-ladder-sim.pro/audit_trail.jsonl` |
| Linux | `~/.local/share/com.plc-ladder-sim.pro/audit_trail.jsonl` |
| Windows | `%APPDATA%\com.plc-ladder-sim.pro\audit_trail.jsonl` |

## 7. Troubleshooting

| Symptom | Action |
|---------|--------|
| Port 5020 in use | Free the port or change default in `src-tauri/src/plc/modbus.rs` |
| UI shows mock / no live scan | Use `npm run tauri:dev`, not Vite alone |
| Blank window | Check Vite port **1420** conflicts |
| macOS Gatekeeper on downloaded app | Unsigned OSS builds — see [RELEASES.md](./RELEASES.md) |

## Version matrix

| Component | Target |
|-----------|--------|
| Tauri | 2.x |
| Svelte | 5.x |
| Vite | 6.x |
| tokio-modbus | 0.16.x |
| Rust edition | 2021 |
