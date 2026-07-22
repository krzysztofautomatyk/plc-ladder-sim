<script lang="ts">
  import { plc } from "../../../shared/stores/plc.svelte";
  import type { Address, LadderElement } from "../../../shared/lib/types";
  import { readMemoryBit } from "../lib/memoryRead";
  import { kindForKey } from "../lib/shortcuts";
  import type { MoveDir } from "../lib/ladderMove";
  import RungView from "./LadderNetwork.svelte";
  import LadderInsertToolbar from "./LadderInsertToolbar.svelte";

  function isEnergized(addr: Address): boolean {
    return readMemoryBit(plc.memory, addr);
  }

  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (plc.view !== "ladder") return;
    const t = e.target as HTMLElement | null;
    if (t) {
      const tag = t.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || t.isContentEditable)
        return;
    }
    const kind = kindForKey(e.key);
    if (kind) {
      e.preventDefault();
      plc.insertInstruction(kind);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="editor-root">
  <div class="editor-toolbar">
    <div class="block-title">
      <span class="ob">OB1</span>
      <span class="sep">›</span>
      <strong>{plc.program.name}</strong>
      <span class="lang">LAD</span>
      <span class="hint" style="margin-left:8px"
        >{plc.program.rungs.length} networks</span
      >
    </div>
    <input class="ver" bind:value={plc.program.version} title="Version" />
    <button type="button" class="tia-btn" onclick={() => plc.addRung()}>+ Network</button>
    <button type="button" class="tia-btn tia-btn-primary" onclick={() => plc.pushProgram()}
      >Download to PLC</button
    >
    <span class="hint">IEC 61131-3 · left rail → right rail · online monitor</span>
  </div>

  <LadderInsertToolbar
    onInsert={(k) => plc.insertInstruction(k)}
    onInsertParallel={() => plc.addParallelBlock()}
    onAddNetwork={() => plc.addRung()}
    onDeleteNetwork={() => plc.deleteSelectedNetwork()}
    targetLabel={plc.insertTargetLabel}
  />

  <div class="editor-canvas">
    {#if plc.program.rungs.length === 0}
      <div class="empty">
        <div class="empty-card">
          <h3>Empty organization block</h3>
          <p>Add a network and insert contacts, coils, and function blocks from the toolbar above (or press keys 1–8, o).</p>
          <button type="button" class="tia-btn tia-btn-primary" onclick={() => plc.addRung()}
            >Insert network</button
          >
        </div>
      </div>
    {:else}
      {#each plc.program.rungs as rung, i (rung.id)}
        <RungView
          {rung}
          networkNo={i}
          selected={plc.selectedRungId === rung.id}
          selectedBranch={plc.selectedRungId === rung.id ? plc.selectedBranch : null}
          selectedParallel={plc.selectedRungId === rung.id ? plc.selectedParallel : null}
          active={plc.isRungActive(rung.id)}
          isElementActive={(id) => plc.isActive(id)}
          isEnergized={isEnergized}
          labelFor={(id, el) => (el ? plc.labelForElement(el) : plc.labelFor(id))}
          online={plc.status?.run_state === "run" || plc.status?.running === true}
          onSelect={() => plc.selectRung(rung.id)}
          onSelectBranch={(bi) => plc.selectBranch(rung.id, bi)}
          onSelectParallelBranch={(gid, bi) => plc.selectParallelBranch(rung.id, gid, bi)}
          onAddParallelBranch={(gid) => plc.addBranchToParallel(rung.id, gid)}
          onRemoveParallelBranch={(gid, bi) => plc.removeParallelBranch(rung.id, gid, bi)}
          onRemoveParallelGroup={(gid) => plc.removeParallelGroup(rung.id, gid)}
          onRemove={() => plc.removeRung(rung.id)}
          onComment={(c) => plc.updateRungComment(rung.id, c)}
          onAddKind={(k) => plc.addElement(rung.id, k)}
          onRemoveElement={(id) => plc.removeElement(rung.id, id)}
          onMove={(id, dir: MoveDir) => plc.moveInRung(rung.id, id, dir)}
          canMove={(id, dir: MoveDir) => plc.canMoveInRung(rung.id, id, dir)}
          onChangeElement={(el) => plc.updateElement(rung.id, el)}
          onAddOrBranch={() => plc.addOrBranch(rung.id)}
          onRemoveOrBranch={(bi) => plc.removeOrBranch(rung.id, bi)}
          onAddToOrBranch={(bi, k) => plc.addToOrBranch(rung.id, bi, k)}
          onRemoveFromOrBranch={(bi, id) => plc.removeFromOrBranch(rung.id, bi, id)}
          onChangeOrElement={(bi, el) => plc.updateOrElement(rung.id, bi, el)}
          onEditElement={(el: LadderElement, orBranch: number | null) =>
            plc.openElementEditor(rung.id, el, orBranch)}
        />
      {/each}
      <div class="canvas-end">— end of OB1 —</div>
    {/if}
  </div>
</div>

<style>
  .editor-root {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #d8dde3;
    overflow: hidden;
  }
  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: linear-gradient(180deg, #f4f6f8, #e6ebf0);
    border-bottom: 1px solid #b0b8c0;
    flex-shrink: 0;
    z-index: 5;
  }
  .block-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }
  .ob {
    background: #005f87;
    color: #fff;
    font-weight: 700;
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 2px;
  }
  .sep {
    color: #8a949e;
  }
  .lang {
    font-size: 10px;
    font-weight: 700;
    color: #005f87;
    border: 1px solid #8ab;
    padding: 0 4px;
    border-radius: 2px;
  }
  .ver {
    width: 52px;
    border: 1px solid #b0b8c0;
    padding: 2px 4px;
    font-size: 12px;
  }
  .hint {
    margin-left: auto;
    font-size: 11px;
    color: #6a7682;
  }
  .editor-canvas {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    padding: 10px 0 24px;
    /* Only this region scrolls — OB1 bar + insert strip stay pinned above */
  }
  .canvas-end {
    text-align: center;
    font-size: 11px;
    color: #8a949e;
    padding: 12px;
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 200px;
  }
  .empty-card {
    background: #fff;
    border: 1px solid #c0c7ce;
    padding: 24px 32px;
    text-align: center;
    max-width: 360px;
  }
  .empty-card h3 {
    margin: 0 0 8px;
    font-size: 14px;
  }
  .empty-card p {
    margin: 0 0 16px;
    font-size: 12px;
    color: #6a7682;
  }
</style>
