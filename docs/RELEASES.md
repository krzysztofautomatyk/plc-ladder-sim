# Releases & installers

Prebuilt installers for **Windows**, **macOS**, and **Linux** are produced by GitHub Actions and attached to [GitHub Releases](https://github.com/YOUR_GITHUB_USER/plc-ladder-sim/releases).

## Download matrix (v1)

| Platform | Installer types |
|----------|-----------------|
| **Windows** | `.msi`, `.exe` (NSIS) |
| **macOS** | `.dmg` |
| **Linux** | `.deb`, `.AppImage` |

Exact file names include the version, e.g. `PLC Ladder Simulator Pro_1.0.0_…`.

## How CI builds them

Workflow: [`.github/workflows/release.yml`](../.github/workflows/release.yml)

| Trigger | Effect |
|---------|--------|
| Push tag `v*` (e.g. `v1.0.0`) | Build matrix → draft GitHub Release + assets |
| `workflow_dispatch` | Manual run (for maintainers) |

Matrix runners:

- `windows-latest`
- `macos-latest`
- `ubuntu-22.04`

Uses [`tauri-apps/tauri-action`](https://github.com/tauri-apps/tauri-action).

## Create a release (maintainers)

```bash
# on main, clean tree
git tag v1.0.0
git push origin v1.0.0
```

1. Wait for the **Release** workflow (all 3 OS jobs)
2. Open the **draft** release on GitHub
3. Review assets → **Publish release**

Version should match `package.json` / `src-tauri/tauri.conf.json` / `Cargo.toml` (`1.0.0` ↔ tag `v1.0.0`).

## Unsigned builds (v1)

v1 installers are **not** Apple-notarized and **not** Windows Authenticode-signed unless maintainers add certificates later.

| OS | What you may see |
|----|------------------|
| macOS | Gatekeeper: “app can’t be opened” / unidentified developer → right-click → Open, or System Settings → Privacy |
| Windows | SmartScreen: “Windows protected your PC” → More info → Run anyway |
| Linux | Usually fine; mark AppImage executable: `chmod +x *.AppImage` |

## Local production build

```bash
npm install
npm run tauri:build
```

Output under `src-tauri/target/release/bundle/` (platform-specific subfolders: `dmg`, `msi`, `deb`, …).

## Auto-updater

Not enabled in v1.0. Future work may add Tauri updater + signed artifacts.
