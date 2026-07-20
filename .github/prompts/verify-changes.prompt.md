---
mode: agent
description: "Run every quality gate (frontend + Rust) and report a concise pass/fail summary."
---

# Verify changes

Run the project's full quality gates and report a short pass/fail table. Do **not**
change source code — only run commands and summarise. If something fails, show the
minimal failing output and propose a fix.

Frontend (repo root):

```bash
npm run check   # svelte-check — expect 0 errors, 0 warnings
npm run test    # Vitest
npm run build   # production build
```

Backend (inside `src-tauri/`):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Report: for each command, ✅/❌, and the key numbers (test counts, warnings). Treat any
clippy warning or svelte-check warning as a failure.
