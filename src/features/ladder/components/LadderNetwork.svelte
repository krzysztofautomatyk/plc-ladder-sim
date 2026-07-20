<script lang="ts">
  /**
   * TIA-style network (9/10 target):
   * - dense series cells
   * - coils right-justified against right rail
   * - continuous power-flow coloring
   * - OR branches above series
   */
  import type {
    Address,
    LadderElement,
    PaletteKind,
    RungNode,
    Rung as RungType,
  } from "../../../shared/lib/types";
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
    /** Which OR branch is the current insert target (null = main series). */
    selectedBranch?: number | null;
    /** Which inline parallel-group branch is the current insert target. */
    selectedParallel?: { groupId: string; branch: number } | null;
    active: boolean;
    isElementActive: (id: string) => boolean;
    /** Live bit state for blue SET/coil highlight */
    isEnergized?: (addr: Address) => boolean;
    /** Symbolic label lookup for an element id. */
    labelFor?: (id: string) => string;
    online?: boolean;
    onSelect: () => void;
    onSelectBranch: (branchIdx: number) => void;
    onSelectParallelBranch: (groupId: string, branchIdx: number) => void;
    onAddParallelBranch: (groupId: string) => void;
    onRemoveParallelBranch: (groupId: string, branchIdx: number) => void;
    onRemoveParallelGroup: (groupId: string) => void;
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
    selectedBranch = null,
    selectedParallel = null,
    active,
    isElementActive,
    isEnergized = () => false,
    labelFor = () => "",
    online = false,
    onSelect,
    onSelectBranch,
    onSelectParallelBranch,
    onAddParallelBranch,
    onRemoveParallelBranch,
    onRemoveParallelGroup,
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

  /** True when at least one parallel OR branch fully conducts. */
  const orMerges = $derived(
    (rung.or_branches ?? []).some((b) => b.length > 0 && pout(b, b.length - 1))
  );

  /** Power leaving the OR merge (or the left rail when there is no OR block). */
  const feedLeft = $derived(rung.or_branches?.length ? online && orMerges : online);

  /** Power at end of left logic (feeds all parallel coils) */
  const feedCoils = $derived(
    online &&
      (split.left.length
        ? pout(split.left, split.left.length - 1)
        : rung.or_branches?.length
          ? orMerges
          : true)
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

  function nodeConducts(n: RungNode): boolean {
    if (n.type === "parallel") {
      return n.branches.some((b) => b.length > 0 && b.every((e) => isElementActive(e.id)));
    }
    return isElementActive(n.id);
  }
  function pin(els: RungNode[], i: number) {
    return seriesPowerIn(els, i, nodeConducts);
  }
  function pout(els: RungNode[], i: number) {
    return seriesPowerOut(els, i, nodeConducts);
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
      <div class="bus main">
        <div class="flow">
          {#if series.length === 0 && !rung.or_branches?.length}
            <div class="placeholder">Press 1–8 or use the toolbar to insert an instruction…</div>
          {:else}
            <!-- feed from left rail (always live in RUN) -->
            <i class="seg s0" class:hot={online}></i>

            {#if rung.or_branches?.length}
              <!-- Inline parallel OR block (contacts tied between two vertical buses) -->
              <div
                class="par"
                role="group"
                aria-label="Parallel OR block"
                ondrop={(e) => onBranchDrop(e, 0)}
                ondragover={onDragOver}
              >
                <div class="par-lbus" class:hot={online}></div>
                <div class="par-rows">
                  {#each rung.or_branches as branch, bi (bi)}
                    {@const bActive = branch.some((e) => isElementActive(e.id))}
                    <div
                      class="par-row"
                      class:sel={selectedBranch === bi}
                      ondrop={(e) => onBranchDrop(e, bi)}
                      ondragover={onDragOver}
                      role="group"
                      aria-label={`OR branch ${bi}`}
                    >
                      <button
                        type="button"
                        class="branch-tab"
                        class:sel={selectedBranch === bi}
                        title="Select this OR branch as insert target"
                        onclick={(e) => {
                          e.stopPropagation();
                          onSelectBranch(bi);
                        }}>OR{bi}</button
                      >
                      <i class="seg jin" class:hot={online || bActive}></i>
                      {#each branch as el, ei (el.id)}
                        {#if ei > 0}
                          <i class="seg" class:hot={pout(branch, ei - 1)}></i>
                        {/if}
                        <LadderElementHost
                          element={el}
                          label={labelFor(el.id)}
                          active={isElementActive(el.id)}
                          powerIn={online && pin(branch, ei)}
                          energized={energ(el)}
                          compact
                          onRemove={() => onRemoveFromOrBranch(bi, el.id)}
                          onChange={(e) => onChangeOrElement(bi, e)}
                          onEdit={() => onEditElement(el, bi)}
                        />
                      {/each}
                      <i
                        class="seg jout"
                        class:hot={branch.length ? pout(branch, branch.length - 1) : false}
                      ></i>
                      <button
                        type="button"
                        class="btn tiny rm"
                        title="Remove branch"
                        onclick={(e) => {
                          e.stopPropagation();
                          onRemoveOrBranch(bi);
                        }}>✕</button
                      >
                    </div>
                  {/each}
                </div>
                <div class="par-rbus" class:hot={online && orMerges}></div>
              </div>
              <i class="seg mergeout" class:hot={feedLeft}></i>
            {/if}

            {#each split.left as node, ei (node.id)}
              {#if ei > 0}
                <i class="seg" class:hot={pout(split.left, ei - 1)}></i>
              {/if}
              {#if node.type === "parallel"}
                {@const pfeed = ei === 0 ? feedLeft : online && pin(split.left, ei)}
                <div class="par inline" role="group" aria-label="Parallel group">
                  <div class="par-lbus" class:hot={pfeed}></div>
                  <div class="par-rows">
                    {#each node.branches as branch, bi (bi)}
                      {@const psel =
                        !!selectedParallel &&
                        selectedParallel.groupId === node.id &&
                        selectedParallel.branch === bi}
                      <div class="par-row" class:sel={psel}>
                        <button
                          type="button"
                          class="branch-tab"
                          class:sel={psel}
                          title="Select this parallel branch as insert target"
                          onclick={(e) => {
                            e.stopPropagation();
                            onSelectParallelBranch(node.id, bi);
                          }}>∥{bi}</button
                        >
                        <i class="seg jin" class:hot={pfeed}></i>
                        {#each branch as el, ci (el.id)}
                          {#if ci > 0}
                            <i class="seg" class:hot={isElementActive(branch[ci - 1].id)}></i>
                          {/if}
                          <LadderElementHost
                            element={el}
                            label={labelFor(el.id)}
                            active={isElementActive(el.id)}
                            powerIn={pfeed && (ci === 0 || isElementActive(branch[ci - 1].id))}
                            energized={energ(el)}
                            compact
                            onRemove={() => onRemoveElement(el.id)}
                            onChange={onChangeElement}
                            onEdit={() => onEditElement(el, null)}
                          />
                        {/each}
                        <i
                          class="seg jout"
                          class:hot={branch.length
                            ? isElementActive(branch[branch.length - 1].id)
                            : false}
                        ></i>
                        <button
                          type="button"
                          class="btn tiny rm"
                          title="Remove this branch"
                          onclick={(e) => {
                            e.stopPropagation();
                            onRemoveParallelBranch(node.id, bi);
                          }}>✕</button
                        >
                      </div>
                    {/each}
                    <button
                      type="button"
                      class="add-branch"
                      title="Add a parallel branch"
                      onclick={(e) => {
                        e.stopPropagation();
                        onAddParallelBranch(node.id);
                      }}>＋ branch</button
                    >
                  </div>
                  <div class="par-rbus" class:hot={online && nodeConducts(node)}></div>
                </div>
              {:else}
                <LadderElementHost
                  element={node}
                  label={labelFor(node.id)}
                  active={isElementActive(node.id)}
                  powerIn={ei === 0 ? feedLeft : online && pin(split.left, ei)}
                  energized={energ(node)}
                  onRemove={() => onRemoveElement(node.id)}
                  onChange={onChangeElement}
                  onEdit={() => onEditElement(node, null)}
                />
              {/if}
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
                      label={labelFor(el.id)}
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

  /* Inline parallel OR block: rows between a left bus and a merge bus */
  .par {
    display: flex;
    align-items: stretch;
    flex-shrink: 0;
    align-self: center;
  }
  .par-lbus,
  .par-rbus {
    width: 2px;
    background: #1a1a1a;
    flex-shrink: 0;
    align-self: stretch;
    z-index: 1;
  }
  .par-lbus.hot,
  .par-rbus.hot {
    background: #00a651;
    box-shadow: 0 0 3px rgba(0, 166, 81, 0.45);
  }
  .par-rows {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }
  .par-row {
    position: relative;
    display: flex;
    align-items: center;
    min-height: 84px;
  }
  .par-row.sel {
    background: rgba(0, 120, 168, 0.08);
    outline: 1px dashed #0078a8;
    outline-offset: -2px;
  }
  .par-row .seg.jin {
    width: 10px;
  }
  .par-row .seg.jout {
    width: 10px;
  }
  .branch-tab {
    position: absolute;
    left: 2px;
    top: 2px;
    z-index: 5;
    font-size: 9px;
    font-weight: 700;
    font-family: Consolas, monospace;
    color: #5a6570;
    background: #e8eef2;
    border: 1px solid #c5ccd2;
    border-radius: 2px;
    padding: 0 4px;
    cursor: pointer;
    line-height: 1.4;
  }
  .branch-tab:hover {
    border-color: #0078a8;
    color: #005f87;
  }
  .branch-tab.sel {
    background: #0078a8;
    color: #fff;
    border-color: #005f87;
  }
  .btn.tiny.rm {
    align-self: center;
  }
  .seg.mergeout {
    width: 16px;
  }
  .par.inline {
    align-self: center;
  }
  .add-branch {
    align-self: flex-start;
    margin: 2px 0 0 20px;
    font-size: 9px;
    font-family: Consolas, monospace;
    border: 1px dashed #9aa3ab;
    background: #f4f7f9;
    color: #5a6570;
    border-radius: 2px;
    padding: 1px 6px;
    cursor: pointer;
    line-height: 1.4;
  }
  .add-branch:hover {
    border-color: #0078a8;
    color: #005f87;
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
