# Contributing

Thanks for helping improve **PLC Ladder Simulator Pro**.

## Development setup

See [docs/SETUP.md](./docs/SETUP.md).

```bash
npm install
npm run tauri:dev
```

## Checks before a PR

```bash
# Frontend
npm run build
npm run check   # optional type check

# Rust core
cd src-tauri && cargo test && cd ..
```

## Project conventions

- **One IEC ladder instruction = one folder** under `src/features/ladder/elements/<name>/` with `definition.ts` + `glyph.svelte`. Register it in `elements/_shared/registry.ts`.
- Prefer small, focused PRs over large mixed refactors.
- Do not commit `node_modules/`, `src-tauri/target/`, or secrets.

## Pull requests

1. Fork and create a feature branch from `main`
2. Make your change with a clear description
3. Ensure build + tests pass
4. Open a PR against `main`

## Releases / installers

Installers for Windows, macOS, and Linux are built by GitHub Actions on version tags (`v*`). See [docs/RELEASES.md](./docs/RELEASES.md).

## Code of conduct

By participating you agree to follow our [Code of Conduct](./CODE_OF_CONDUCT.md).
