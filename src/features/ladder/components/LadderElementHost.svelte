<script lang="ts">
  /**
   * Host cell for one ladder instruction: labels + click chrome + glyph from registry.
   * Each instruction lives in ../elements/<name>/{definition,glyph}.
   */
  import type { LadderElement } from "../../../shared/lib/types";
  import { formatAddress } from "../lib/addressFormat";
  import { resolveStrokes } from "../lib/strokeColors";
  import {
    getRegistryEntry,
    isCoilType,
    isFbType,
  } from "../elements";

  interface Props {
    element: LadderElement;
    active: boolean;
    powerIn?: boolean;
    energized?: boolean;
    compact?: boolean;
    onRemove: () => void;
    onChange: (el: LadderElement) => void;
    onEdit?: () => void;
  }

  let {
    element,
    active,
    powerIn = false,
    energized = false,
    compact = false,
    onRemove,
    onEdit,
  }: Props = $props();

  const entry = $derived(getRegistryEntry(element.type));
  const def = $derived(entry.def);
  const Glyph = $derived(entry.Glyph);

  const isCoil = $derived(isCoilType(element.type));
  const isFb = $derived(isFbType(element.type));

  const strokes = $derived(
    resolveStrokes({
      active,
      powerIn,
      energized,
      isCoil,
    })
  );

  const lit = $derived(active || energized);

  const topLabel = $derived(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    def.topLabel(element as any, formatAddress)
  );
  const bottomLabel = $derived(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    def.bottomLabel(element as any, formatAddress)
  );

  function openEdit(e: Event) {
    e.preventDefault();
    e.stopPropagation();
    onEdit?.();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div
  class="cell"
  class:lit
  class:energized
  class:fb={isFb}
  class:compact
  class:wire-only={element.type === "wire"}
  class:coil={isCoil}
  class:edge={def.cellClass === "edge"}
  onpointerdown={openEdit}
  onclick={openEdit}
  ondblclick={openEdit}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") openEdit(e);
  }}
  role="button"
  tabindex="0"
  title="Kliknij, aby edytować adres (I/Q/M/R)"
>
  <button
    type="button"
    class="del"
    onclick={(e) => {
      e.stopPropagation();
      onRemove();
    }}
    onpointerdown={(e) => e.stopPropagation()}
    title="Delete">×</button
  >

  {#if topLabel && element.type !== "wire"}
    <div class="label top" class:hot={lit}>{topLabel}</div>
  {/if}

  <div class="glyph">
    <Glyph
      {element}
      strokeIn={strokes.strokeIn}
      strokeOut={strokes.strokeOut}
      strokeBody={strokes.strokeBody}
      fillCoil={strokes.fillCoil}
      sw={strokes.sw}
      {active}
    />
  </div>

  {#if bottomLabel}
    <div class="label bot" class:hot={lit}>{bottomLabel}</div>
  {/if}
</div>

<style>
  .cell {
    position: relative;
    height: 92px;
    width: 64px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2;
    cursor: pointer;
    border-radius: 2px;
  }
  .cell:hover {
    outline: 1px dashed #0078a8;
    outline-offset: 1px;
  }
  .cell:focus-visible {
    outline: 2px solid #0078a8;
  }
  .cell.fb {
    width: 100px;
  }
  .cell.coil {
    width: 64px;
  }
  .cell.wire-only {
    width: 32px;
  }
  .cell.edge {
    width: 72px;
  }
  .cell.compact {
    height: 84px;
  }

  .del {
    position: absolute;
    top: 0;
    right: 0;
    z-index: 6;
    border: 1px solid #ccc;
    background: #fff;
    color: #888;
    width: 15px;
    height: 15px;
    font-size: 10px;
    line-height: 1;
    padding: 0;
    opacity: 0;
    cursor: pointer;
    border-radius: 2px;
  }
  .cell:hover .del {
    opacity: 1;
  }
  .del:hover {
    color: #c00000;
    border-color: #c00000;
  }

  .label {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    font-family: Consolas, "Cascadia Mono", "Courier New", monospace;
    font-size: 11px;
    font-weight: 700;
    color: #003d5c;
    white-space: nowrap;
    max-width: 98px;
    overflow: hidden;
    text-overflow: ellipsis;
    z-index: 3;
    line-height: 1.15;
  }
  .label.top {
    bottom: calc(50% + 18px);
  }
  .label.bot {
    top: calc(50% + 18px);
    font-size: 9.5px;
    font-weight: 500;
    color: #5a6570;
  }
  .label.hot {
    color: #007a3d;
  }
  .cell.energized .label.top {
    color: #1565c0;
    font-weight: 800;
  }
  .cell.energized.coil {
    filter: drop-shadow(0 0 4px rgba(21, 101, 192, 0.55));
  }

  .glyph {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 36px;
    background: transparent;
    z-index: 2;
  }
</style>
