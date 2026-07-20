---
mode: agent
description: "Scaffold a new IEC ladder instruction package (definition + glyph + registry)."
---

# Add a new ladder instruction

Add a new IEC ladder instruction named `${input:name:e.g. shift_left}` to the frontend.

Follow the existing instruction-package pattern exactly:

1. Create `src/features/ladder/elements/${input:name}/`:
   - `definition.ts` — export an `ElementDefinition<"${input:name}">` with `kind`,
     `category`, `label`, `shortLabel`, `help`, `cellClass`, a `create()` factory, and
     `topLabel`/`bottomLabel`. Mirror a similar existing element.
   - `glyph.svelte` — a **pure** SVG glyph using `ElementRenderProps`. No store access,
     no click handlers. Reuse `FunctionBlockBox` for function-block style elements.
   - `index.ts` — re-export `definition` (as `Glyph`/definition per the shared type).
2. Add the variant to the `LadderElement` union in `src/shared/lib/types.ts`.
3. Register it with **one line** in `elements/_shared/registry.ts` and add it to the
   palette group + `PALETTE_ORDER`.
4. Backend: add the matching `LadderElement` variant in
   `src-tauri/src/plc/compiler.rs` and handle it in `compile_elements` (emit the right
   bytecode). Add a compiler test and, if it has runtime behaviour, an engine test.

Then run the full quality gates and report results:
`npm run check && npm run test && npm run build`, and in `src-tauri/`:
`cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`.

Keep the change surgical and consistent with the surrounding code.
