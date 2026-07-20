<script lang="ts">
  /**
   * TIA-style network (9/10 target):
   * - dense series cells
   * - coils right-justified against right rail
   * - continuous power-flow coloring
   * - OR branches above series
   */
  import type { Address, LadderElement, PaletteKind, Rung as RungType } from "../../../shared/lib/types";
  import LadderElementHost from "./LadderElementHost.svelte";
  import {
    seriesPowerIn,
    seriesPowerOut,
    splitRungElements,
  } from "../lib/ladderLayout";

  interface Props {
    rung: RungType;
    networkNo: number;
    selected: boolean;
    active: boolean;
    isElementActive: (id: string) => boolean;
    /** Live bit state for blue SET/coil highlight */
    isEnergized?: (addr: Address) => boolean;
    online?: boolean;
    onSelect: () => void;
    onRemove: () => void;
    onComment: (c: string) => void;
    onAddKind: (kind: PaletteKind) => void;
    onRemoveElement: (id: string) => void;
    onChangeElement: (el: LadderElement) => void;
    onAddOrBranch: () => void;
    onRemoveOrBranch: (branchIdx: number) => void;
    onAddToOrBranch: (branchIdx: number, kind: PaletteKind) => void;
    onRemoveFromOrBranch: (branchIdx: number, elementId: string) => void;
    onChangeOrElement: (branchIdx: number, el: LadderElement) => void;
    onEditElement: (el: LadderElement, orBranch: number | null) => void;
  }

  let {
    rung,
    networkNo,
    selected,
    active,
    isElementActive,
    isEnergized = () => false,
    online = false,
    onSelect,
    onRemove,
    onComment,
    onAddKind,
    onRemoveElement,
    onChangeElement,
    onAddOrBranch,
    onRemoveOrBranch,
    onAddToOrBranch,
    onRemoveFromOrBranch,
    onChangeOrElement,
    onEditElement,
  }: Props = $props();

  const split = $derived(splitRungElements(rung.elements ?? []));

  /** Full series order for power: left then right (coil last) */
  const series = $derived([...split.left, ...split.right]);

  const anyActive = $derived(
    series.some((e) => isElementActive(e.id)) ||
      (rung.or_branches ?? []).some((b) => b.some((e) => isElementActive(e.id)))
  );

  /** Right rail hot if any output coil is active */
  const rightPower = $derived(split.right.some((e) => isElementActive(e.id)));

  /** Power at end of left logic (feeds all parallel coils) */
  const feedCoils = $derived(
    online &&
      (split.left.length ? pout(split.left, split.left.length - 1) : true)
  );

  function onDrop(e: DragEvent) {
    e.preventDefault();
    const kind = e.dataTransfer?.getData("application/x-lad-kind") as PaletteKind | undefined;
    if (kind) onAddKind(kind);
  }
  function onDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  }
  function onBranchDrop(e: DragEvent, bi: number) {
    e.preventDefault();
    e.stopPropagation();
    const kind = e.dataTransfer?.getData("application/x-lad-kind") as PaletteKind | undefined;
    if (kind && kind !== "or_branch") onAddToOrBranch(bi, kind);
  }

  function pin(els: LadderElement[], i: number) {
    return seriesPowerIn(els, i, isElementActive);
  }
  function pout(els: LadderElement[], i: number) {
    return seriesPowerOut(els, i, isElementActive);
  }
  function energ(el: LadderElement): boolean {
    if ("address" in el && el.address) return isEnergized(el.address);
    return false;
  }
</script>

<div
  class="net"
  class:selected
  class:online={active || anyActive}
  ondrop={onDrop}
  ondragover={onDragOver}
  role="group"
>
  <header class="hdr">
    <button type="button" class="badge" aria-pressed={selected} onclick={onSelect}>
      Network {networkNo}
    </button>
    <input
      class="title"
      value={rung.comment}
      oninput={(e) => onComment(e.currentTarget.value)}
      onclick={(e) => e.stopPropagation()}
      placeholder="Network comment…"
    />
    <button
      type="button"
      class="btn"
      onclick={(e) => {
        e.stopPropagation();
        onAddOrBranch();
      }}
      title="Parallel OR branch">∥ OR</button
    >
    <button
      type="button"
      class="btn danger"
      onclick={(e) => {
        e.stopPropagation();
        onRemove();
      }}>✕</button
    >
  </header>

  <div class="body">
    <!-- Left power rail: energized when PLC online -->
    <div class="rail" class:hot={online || anyActive}></div>

    <div class="center">
      {#if rung.or_branches?.length}
        <div class="or-block">
          {#each rung.or_branches as branch, bi (bi)}
            {@const bActive = branch.some((e) => isElementActive(e.id))}
            <div
              class="bus or-bus"
              ondrop={(e) => onBranchDrop(e, bi)}
              ondragover={onDragOver}
              role="group"
              aria-label={`OR branch ${bi}`}
            >
              <span class="or-id">OR{bi}</span>
              <div class="flow">
                <i class="seg s0" class:hot={online || bActive}></i>
                {#each branch as el, ei (el.id)}
                  {#if ei > 0}
                    <i class="seg" class:hot={pout(branch, ei - 1)}></i>
                  {/if}
                  <LadderElementHost
                    element={el}
                    active={isElementActive(el.id)}
                    powerIn={pin(branch, ei)}
                    energized={energ(el)}
                    compact
                    onRemove={() => onRemoveFromOrBranch(bi, el.id)}
                    onChange={(e) => onChangeOrElement(bi, e)}
                    onEdit={() => onEditElement(el, bi)}
                  />
                {/each}
                <i class="seg s1" class:hot={branch.length ? pout(branch, branch.length - 1) : false}
                ></i>
                <button
                  type="button"
                  class="btn tiny"
                  onclick={(e) => {
                    e.stopPropagation();
                    onRemoveOrBranch(bi);
                  }}>✕</button
                >
              </div>
            </div>
          {/each}
          <div class="or-join">
            <span>OR → series</span>
          </div>
        </div>
      {/if}

      <!-- Main series: logic LEFT · grow wire · coils RIGHT (TIA) -->
      <div class="bus main">
        <div class="flow">
          {#if series.length === 0}
            <div class="placeholder">Drop contacts / coils here</div>
          {:else}
            <!-- feed from left rail (always live in RUN) -->
            <i class="seg s0" class:hot={online}></i>

            {#each split.left as el, ei (el.id)}
              {#if ei > 0}
                <i class="seg" class:hot={pout(split.left, ei - 1)}></i>
              {/if}
              <LadderElementHost
                element={el}
                active={isElementActive(el.id)}
                powerIn={online && pin(split.left, ei)}
                energized={energ(el)}
                onRemove={() => onRemoveElement(el.id)}
                onChange={onChangeElement}
                onEdit={() => onEditElement(el, null)}
              />
            {/each}

            <!-- Long run to right-hand coil column (TIA) -->
            <i class="seg grow" class:hot={feedCoils}></i>

            {#if split.right.length === 0}
              <i class="seg s1" class:hot={feedCoils}></i>
            {:else}
              <!-- Multiple outputs stacked vertically (one under another) -->
              <div class="coil-stack">
                <div class="coil-vbus" class:hot={feedCoils} aria-hidden="true"></div>
                {#each split.right as el, ri (el.id)}
                  <div class="coil-row">
                    <i class="seg coil-in" class:hot={feedCoils}></i>
                    <LadderElementHost
                      element={el}
                      active={isElementActive(el.id)}
                      powerIn={feedCoils}
                      energized={energ(el)}
                      onRemove={() => onRemoveElement(el.id)}
                      onChange={onChangeElement}
                      onEdit={() => onEditElement(el, null)}
                    />
                    <i
                      class="seg s1"
                      class:hot={isElementActive(el.id) || energ(el)}
                      class:blue={energ(el) && !isElementActive(el.id)}
                    ></i>
                  </div>
                  {#if ri < split.right.length - 1}
                    <div class="coil-gap" aria-hidden="true"></div>
                  {/if}
                {/each}
              </div>
            {/if}
          {/if}
        </div>
      </div>
    </div>

    <div class="rail" class:hot={rightPower || (online && series.length === 0)}></div>
  </div>
</div>

<style>
  .net {
    margin: 0 10px 12px;
    background: #fff;
    border: 1px solid #a0a8b0;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  }
  .net.selected {
    border-color: #0078a8;
    box-shadow: 0 0 0 1px #0078a8;
  }
  .net.online {
    border-color: #5aad7a;
  }

  .hdr {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: linear-gradient(180deg, #eef2f6, #e0e6eb);
    border-bottom: 1px solid #c0c7ce;
  }
  .badge {
    border: 0;
    font-size: 11px;
    font-weight: 700;
    color: #fff;
    background: linear-gradient(180deg, #2a88b8, #005f87);
    padding: 2px 8px;
    border-radius: 2px;
    flex-shrink: 0;
    cursor: pointer;
  }
  .badge:focus-visible {
    outline: 2px solid #003d5c;
    outline-offset: 2px;
  }
  .title {
    flex: 1;
    border: 1px solid transparent;
    background: transparent;
    font-size: 12px;
    padding: 2px 6px;
    min-width: 0;
    color: #222;
  }
  .title:focus {
    outline: none;
    border-color: #9aa3ab;
    background: #fff;
  }
  .btn {
    border: 1px solid #9aa3ab;
    background: linear-gradient(180deg, #fff, #eef1f4);
    font-size: 10px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 2px;
    cursor: pointer;
    color: #222;
  }
  .btn:hover {
    border-color: #0078a8;
  }
  .btn.danger {
    color: #a00;
  }
  .btn.tiny {
    padding: 0 5px;
    margin-left: 4px;
    z-index: 4;
    height: 20px;
  }

  .body {
    display: flex;
    align-items: stretch;
    background:
      repeating-linear-gradient(
        0deg,
        transparent,
        transparent 15px,
        rgba(0, 50, 90, 0.04) 15px,
        rgba(0, 50, 90, 0.04) 16px
      ),
      repeating-linear-gradient(
        90deg,
        transparent,
        transparent 15px,
        rgba(0, 50, 90, 0.04) 15px,
        rgba(0, 50, 90, 0.04) 16px
      ),
      #fafbfc;
    min-height: 100px;
    padding: 4px 0;
  }

  .rail {
    width: 5px;
    flex-shrink: 0;
    background: #1a1a1a;
    margin: 6px 0;
    border-radius: 1px;
    align-self: stretch;
    min-height: 88px;
    z-index: 3;
  }
  .rail:first-child {
    margin-left: 10px;
  }
  .rail:last-child {
    margin-right: 10px;
  }
  .rail.hot {
    background: #00a651;
    box-shadow: 0 0 8px rgba(0, 166, 81, 0.5);
  }

  .center {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .bus {
    position: relative;
    min-height: 92px;
  }
  .bus.main {
    min-height: 96px;
  }

  .flow {
    display: flex;
    align-items: center;
    min-height: 92px;
    width: 100%;
    padding: 0 2px;
  }

  /* Wire segments — single 2px stroke */
  .seg {
    display: block;
    width: 14px;
    height: 2px;
    background: #1a1a1a;
    flex-shrink: 0;
    align-self: center;
  }
  .seg.s0 {
    width: 12px;
  }
  .seg.s1 {
    width: 12px;
  }
  .seg.grow {
    flex: 1 1 auto;
    min-width: 40px;
    width: auto;
  }
  .seg.hot {
    background: #00a651;
    box-shadow: 0 0 3px rgba(0, 166, 81, 0.45);
  }
  .seg.hot.blue {
    background: #1565c0;
    box-shadow: 0 0 4px rgba(21, 101, 192, 0.5);
  }

  .placeholder {
    width: 100%;
    text-align: center;
    font-size: 11px;
    color: #8a949e;
    font-style: italic;
  }

  .or-block {
    margin: 2px 4px 0;
    padding: 2px 0 0;
    border-left: 2px solid #6a7a8a;
    background: rgba(255, 255, 255, 0.55);
  }
  .or-bus {
    position: relative;
    min-height: 84px;
    border-bottom: 1px dotted #c5ccd2;
  }
  .or-bus:last-of-type {
    border-bottom: none;
  }
  .or-id {
    position: absolute;
    left: 2px;
    top: 2px;
    font-size: 9px;
    font-weight: 700;
    font-family: Consolas, monospace;
    color: #5a6570;
    z-index: 5;
    background: #e8eef2;
    padding: 0 3px;
  }
  .or-join {
    text-align: center;
    padding: 2px 0 3px;
  }
  .or-join span {
    font-size: 9px;
    font-family: Consolas, monospace;
    color: #5a6570;
    border: 1px solid #c5ccd2;
    background: #fff;
    padding: 0 6px;
    border-radius: 2px;
  }

  /* Parallel coils — stacked under each other (TIA multi-output) */
  .coil-stack {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    justify-content: center;
    flex-shrink: 0;
    padding: 2px 0;
    min-width: 88px;
  }
  .coil-vbus {
    position: absolute;
    left: 0;
    top: 20px;
    bottom: 20px;
    width: 2px;
    background: #1a1a1a;
    z-index: 0;
  }
  .coil-vbus.hot {
    background: #00a651;
    box-shadow: 0 0 3px rgba(0, 166, 81, 0.4);
  }
  .coil-row {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    min-height: 72px;
  }
  .coil-gap {
    height: 0;
  }
  .seg.coil-in {
    width: 16px;
  }
</style>
