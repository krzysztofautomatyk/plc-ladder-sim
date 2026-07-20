---
applyTo: "src/**/*.ts,src/**/*.svelte"
---

# Frontend (Svelte 5 + TypeScript) instructions

## Framework

- **Svelte 5 runes** only (`$state`, `$derived`, `$effect`, `$props`). The app store
  is a class with `$state` fields (`src/shared/stores/plc.svelte.ts`). Use reactive
  collections from `svelte/reactivity` (e.g. `SvelteSet`) or reassign whole values —
  don't mutate a plain `$state` object and expect deep reactivity for external libs.
- Keep `npm run check` (svelte-check) at **0 errors and 0 warnings**.

## Instruction packages

Each IEC ladder instruction is a folder under `src/features/ladder/elements/<name>/`:

- `definition.ts` — factory (`create`), category, labels, help. Pure data/functions.
- `glyph.svelte` — pure SVG graphic. **No store access, no click logic.**
- `index.ts` — re-exports.

Register the package with **one line** in `elements/_shared/registry.ts`. NO and NC
are separate packages. `ElementType` (registry keys) excludes the `"parallel"` node.

## Rung model

- `Rung.elements` is `RungNode[]` = `LadderElement | { type: "parallel", branches }`.
  `Rung.or_branches` is the leading OR block (seal-in). Do series/parallel edits
  through the **pure, tested** helpers in `features/ladder/lib/ladderEdit.ts` rather
  than mutating arrays inline in the store.
- Coils belong on the output rail only — the store routes them there and blocks them
  from OR/parallel branches. Keep it that way.

## Labels & IPC

- Per-element symbolic labels (max 10 chars) live in `program.metadata["lbl:<id>"]`
  (display-only, round-trips through the backend). Use `plc.labelFor` / `plc.setElementLabel`.
- All backend calls go through `shared/lib/api.ts`, which falls back to a mock when not
  running under Tauri (browser dev via `npm run dev`).

## Testing (Vitest)

Unit-test **pure logic** modules under `features/ladder/lib/` and `shared/lib/`
(addresses, layout, edits, shortcuts). Svelte components/stores need a browser
environment and are covered by type-checking + the Rust engine tests instead.

```bash
npm run check   # svelte-check (0/0)
npm run test    # vitest
npm run build   # production build must succeed
```
