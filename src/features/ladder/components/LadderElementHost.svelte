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
    /** Optional symbolic name shown above the element (e.g. BTN_START). */
    label?: string;
    /** Show ← → move controls (false for wires if desired). */
    canMove?: boolean;
    onRemove: () => void;
    onMoveLeft?: () => void;
    onMoveRight?: () => void;
    onMoveUp?: () => void;
    onMoveDown?: () => void;
    onChange: (el: LadderElement) => void;
    onEdit?: () => void;
  }

  let {
    element,
    active,
    powerIn = false,
    energized = false,
    compact = false,
    label = "",
    canMove = true,
    onRemove,
    onMoveLeft,
    onMoveRight,
    onMoveUp,
    onMoveDown,
    onEdit,
  }: Props = $props();

  const hasAnyMove = $derived(
    !!(onMoveLeft || onMoveRight || onMoveUp || onMoveDown)
  );

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
  /** Left rail stub hot when power arrives; right follows element conduction / coil force. */
  const leftHot = $derived(powerIn || active);
  const rightHot = $derived(
    isCoil ? energized || active : active
  );
  /** Wire stubs bridge cell edge → glyph (glyphs are narrower than the cell). */
  const showThru = $derived(element.type !== "wire");

  const topLabel = $derived(
    def.topLabel(element, formatAddress)
  );
  const bottomLabel = $derived(
    def.bottomLabel(element, formatAddress)
  );

  function openEdit(e: Event) {
    // Never open editor when the user is using move/delete chrome
    const t = e.target as HTMLElement | null;
    if (t?.closest?.("button, .move-pad, .del")) return;
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
  onclick={openEdit}
  ondblclick={openEdit}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") openEdit(e);
  }}
  role="button"
  tabindex="0"
  title="Click to edit · hover for move pad ‹ › ▲ ▼"
>
  {#if canMove && hasAnyMove}
    <!--
      2×2 pad above glyph. Only stopPropagation (never preventDefault on
      pointerdown) so the button click still fires and the cell edit does not open.
    -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="move-pad"
      role="group"
      aria-label="Move element"
      onpointerdown={(e) => e.stopPropagation()}
      onclick={(e) => e.stopPropagation()}
    >
      <span class="mv-spacer"></span>
      <button
        type="button"
        class="mv"
        title="▲ Branch up (OR / ∥) · coil stack up"
        disabled={!onMoveUp}
        onclick={(e) => {
          e.stopPropagation();
          onMoveUp?.();
        }}
        onpointerdown={(e) => e.stopPropagation()}>▲</button
      >
      <span class="mv-spacer"></span>
      <button
        type="button"
        class="mv"
        title="‹ Left · enter leading OR / ∥ · swap in branch"
        disabled={!onMoveLeft}
        onclick={(e) => {
          e.stopPropagation();
          onMoveLeft?.();
        }}
        onpointerdown={(e) => e.stopPropagation()}>‹</button
      >
      <button
        type="button"
        class="mv"
        title="▼ Branch down (OR / ∥) · coil stack down"
        disabled={!onMoveDown}
        onclick={(e) => {
          e.stopPropagation();
          onMoveDown?.();
        }}
        onpointerdown={(e) => e.stopPropagation()}>▼</button
      >
      <button
        type="button"
        class="mv"
        title="› Right · enter ∥ · exit OR at block edge · swap"
        disabled={!onMoveRight}
        onclick={(e) => {
          e.stopPropagation();
          onMoveRight?.();
        }}
        onpointerdown={(e) => e.stopPropagation()}>›</button
      >
    </div>
  {/if}
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

  {#if label}
    <div class="label sym" title={label}>{label}</div>
  {/if}

  {#if topLabel && !isFb && element.type !== "wire"}
    <div class="label top" class:hot={lit} class:has-sym={!!label}>{topLabel}</div>
  {/if}

  <!-- Continuity rails: fill empty space between cell edge and glyph (TIA solid bus) -->
  {#if showThru}
    <i class="thru left" class:hot={leftHot} aria-hidden="true"></i>
    <i class="thru right" class:hot={rightHot} aria-hidden="true"></i>
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

  {#if bottomLabel && !isFb}
    <div class="label bot" class:hot={lit}>{bottomLabel}</div>
  {/if}
</div>

<style>
  .cell {
    /* --gw = glyph body width; rails fill (cell − glyph) / 2 on each side */
    --gw: 56px;
    position: relative;
    height: 110px;
    width: 92px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2;
    cursor: pointer;
    border-radius: 2px;
    overflow: visible;
  }
  .cell:hover {
    outline: 1px dashed #0078a8;
    outline-offset: 1px;
    z-index: 8;
  }
  .cell:focus-visible {
    outline: 2px solid #0078a8;
  }
  .cell.fb {
    --gw: 130px;
    width: 148px;
    height: 132px;
  }
  .cell.coil {
    --gw: 56px;
    width: 88px;
  }
  .cell.wire-only {
    --gw: 28px;
    width: 40px;
  }
  .cell.edge {
    --gw: 64px;
    width: 96px;
  }
  .cell.compact {
    height: 100px;
    width: 88px;
  }
  .cell.compact.coil {
    width: 84px;
  }

  /*
   * Horizontal power rails through the cell.
   * Glyphs are ~56–64px and centered; without these stubs the bus breaks
   * into short dashes between contacts (empty cell margins).
   * +2px overshoots into the glyph so the join is seamless.
   */
  .thru {
    position: absolute;
    top: 50%;
    height: 2px;
    margin-top: -1px;
    background: #1a1a1a;
    z-index: 1;
    pointer-events: none;
  }
  .thru.left {
    left: 0;
    width: calc((100% - var(--gw)) / 2 + 2px);
  }
  .thru.right {
    right: 0;
    width: calc((100% - var(--gw)) / 2 + 2px);
  }
  .thru.hot {
    background: #00a651;
    box-shadow: 0 0 3px rgba(0, 166, 81, 0.45);
  }
  /*
   * Blue = coil/output ON in process image — ONLY on coils.
   * Never paint contact right-stubs blue (that looked like a “blue gap”
   * after a green closed contact).
   */
  .cell.coil.energized .thru.right {
    background: #1565c0;
    box-shadow: 0 0 3px rgba(21, 101, 192, 0.4);
  }

  /* Cross pad — sits fully inside wider cell, › always visible */
  .move-pad {
    position: absolute;
    top: 2px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 7;
    display: grid;
    grid-template-columns: 22px 22px 22px;
    grid-template-rows: 20px 20px;
    gap: 2px;
    opacity: 0;
    pointer-events: none;
    padding: 2px;
    background: rgba(255, 255, 255, 0.95);
    border: 1px solid #c5ccd2;
    border-radius: 3px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.12);
  }
  .mv-spacer {
    width: 22px;
    height: 20px;
  }
  .mv,
  .del {
    border: 1px solid #9aa3ab;
    background: #fff;
    color: #333;
    width: 22px;
    height: 20px;
    font-size: 12px;
    line-height: 1;
    padding: 0;
    cursor: pointer;
    border-radius: 2px;
    font-weight: 800;
    pointer-events: auto;
  }
  .del {
    position: absolute;
    top: 2px;
    right: 2px;
    z-index: 7;
    opacity: 0;
    color: #888;
  }
  .cell:hover .del,
  .cell:hover .move-pad,
  .cell:focus-within .del,
  .cell:focus-within .move-pad {
    opacity: 1;
    pointer-events: auto;
  }
  .mv:hover:not(:disabled) {
    color: #005f87;
    border-color: #0078a8;
    background: #e8f4fc;
  }
  .mv:disabled {
    opacity: 0.3;
    cursor: default;
    color: #aaa;
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
  .label.top.has-sym {
    bottom: calc(50% + 12px);
    font-size: 10px;
  }
  .label.bot {
    top: calc(50% + 18px);
    font-size: 9.5px;
    font-weight: 500;
    color: #5a6570;
  }
  .label.sym {
    bottom: calc(50% + 28px);
    font-size: 10px;
    font-weight: 800;
    color: #6b2d9b;
    letter-spacing: 0.02em;
    max-width: 120px;
    background: rgba(255, 255, 255, 0.92);
    padding: 0 3px;
    border-radius: 2px;
    z-index: 4;
  }
  .cell.fb .label.sym {
    bottom: calc(50% + 52px);
  }
  .cell.coil .label.sym {
    bottom: calc(50% + 28px);
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
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 36px;
    background: transparent;
    z-index: 2;
  }
  .cell.fb .glyph {
    height: auto;
  }
</style>
