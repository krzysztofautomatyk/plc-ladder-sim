<script lang="ts">
  /**
   * Application shell — TIA Portal–style layout.
   * Feature modules live under src/features/*
   */
  import { onMount } from "svelte";
  import { plc } from "../shared/stores/plc.svelte";
  import type { AppView, PaletteKind } from "../shared/lib/types";
  import {
    ElementPalette,
    ElementPropertiesDialog,
    LadderEditor,
  } from "../features/ladder";
  import WatchPanel from "../features/watch/WatchPanel.svelte";
  import SymbolTableView from "../features/symbols/SymbolTableView.svelte";
  import ModbusConfigView from "../features/modbus/ModbusConfigView.svelte";
  import MathHelpView from "../features/docs/MathHelpView.svelte";
  import AuditPanel from "../features/audit/AuditPanel.svelte";
  import LogsView from "../features/diagnostics/LogsView.svelte";
  import MemoryView from "../features/memory/MemoryView.svelte";
  import MemoryAllocationView from "../features/memory/MemoryAllocationView.svelte";

  let fileInput: HTMLInputElement | undefined = $state();
  let treeOpen = $state(true);
  let instrOpen = $state(true);

  onMount(() => {
    plc.init();
    return () => plc.destroy();
  });

  const runState = $derived(plc.status?.run_state ?? "stop");
  const running = $derived(runState === "run" || plc.status?.running === true);

  function onPaletteAdd(kind: PaletteKind) {
    plc.insertInstruction(kind);
  }

  async function onFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (file) await plc.importJsonFile(file);
    input.value = "";
  }

  const nav: { id: AppView; label: string; ico: string; group?: string }[] = [
    { id: "ladder", label: "Main [OB1]", ico: "▦", group: "Program blocks" },
    { id: "tags", label: "PLC tags", ico: "☰", group: "PLC data" },
    { id: "memory", label: "Memory (M/MR)", ico: "▧", group: "PLC data" },
    { id: "alloc", label: "Memory allocation", ico: "▤", group: "PLC data" },
    { id: "math", label: "Math operations", ico: "∑", group: "PLC data" },
    { id: "modbus", label: "Modbus TCP", ico: "⇄", group: "Device config" },
    { id: "audit", label: "Audit trail", ico: "📋", group: "Diagnostics" },
    { id: "logs", label: "Logs", ico: "🗎", group: "Diagnostics" },
  ];
</script>

<div class="tia-shell">
  <div class="tia-menubar">
    <div class="brand">PLC Ladder Simulator <span>TIA-style</span></div>
    <button type="button" onclick={() => plc.loadDemo()}>Project</button>
    <button type="button" onclick={() => plc.pushProgram()}>Edit</button>
    <button type="button" onclick={() => (running ? plc.stop() : plc.start())}>Online</button>
    <button type="button" onclick={() => plc.setView("modbus")}>Tools</button>
  </div>

  <div class="tia-toolbar">
    <button
      type="button"
      class="tia-btn tia-btn-run"
      disabled={running || plc.busy}
      onclick={() => plc.start()}>▶ RUN</button
    >
    <button type="button" class="tia-btn tia-btn-stop" disabled={!running} onclick={() => plc.stop()}
      >■ STOP</button
    >
    <div class="sep"></div>
    <label style="font-size:12px;display:flex;align-items:center;gap:4px;color:var(--tia-muted)">
      Cycle
      <input
        type="number"
        min="5"
        max="100"
        step="5"
        style="width:52px"
        value={plc.cycleMs}
        onchange={(e) => plc.setCycle(Number(e.currentTarget.value))}
      />
      ms
    </label>
    <span
      class="tia-badge"
      class:run={runState === "run"}
      class:stop={runState === "stop"}
      class:fault={runState === "fault"}>{runState.toUpperCase()}</span
    >
    <span class="tia-badge" class:modbus-on={plc.modbus.running} class:modbus-off={!plc.modbus.running}>
      MB {plc.modbus.running ? "ON" : "OFF"} :{plc.modbus.port}
    </span>
    <div class="sep"></div>
    <button type="button" class="tia-btn" onclick={() => plc.loadDemo()}>Demo</button>
    <button type="button" class="tia-btn" onclick={() => plc.exportJson()}>Export</button>
    <button type="button" class="tia-btn" onclick={() => fileInput?.click()}>Import</button>
    <button type="button" class="tia-btn" onclick={() => plc.resetMemory()}>Reset I/O</button>
    <input
      bind:this={fileInput}
      type="file"
      accept="application/json,.json"
      hidden
      onchange={onFile}
    />
    {#if plc.message}
      <span class="msg-bar">{plc.message}</span>
    {/if}
  </div>

  <div class="tia-workspace" style="grid-template-columns: {treeOpen ? '220px' : '32px'} 1fr 280px">
    <aside class="tia-tree">
      {#if treeOpen}
        <div class="tia-tree-title panel-head">
          <span>Project tree</span>
          <button
            type="button"
            class="collapse-btn"
            title="Hide project tree"
            onclick={() => (treeOpen = false)}>◀</button
          >
        </div>
        <div class="tia-tree-group">PLC_1</div>
        {#each nav as item, i}
          {#if item.group && (i === 0 || nav[i - 1].group !== item.group)}
            <div class="tia-tree-group">{item.group}</div>
          {/if}
          <button
            type="button"
            class="tia-tree-item"
            class:active={plc.view === item.id}
            onclick={() => plc.setView(item.id)}
          >
            <span class="ico">{item.ico}</span>
            {item.label}
          </button>
        {/each}
      {:else}
        <button
          type="button"
          class="expand-strip"
          title="Show project tree"
          onclick={() => (treeOpen = true)}>▶ Project tree</button
        >
      {/if}
    </aside>

    <main class="tia-center">
      <div class="tia-center-tabs">
        <button
          type="button"
          class="tab"
          class:active={plc.view === "ladder"}
          onclick={() => plc.setView("ladder")}>Main [OB1] — LAD</button
        >
        {#if plc.view !== "ladder"}
          <button type="button" class="tab active">
            {nav.find((n) => n.id === plc.view)?.label ?? plc.view}
          </button>
        {/if}
      </div>
      <div class="tia-center-body">
        {#if plc.view === "ladder"}
          <div
            style="display:grid;grid-template-columns:{instrOpen
              ? '160px'
              : '32px'} 1fr;height:100%"
          >
            <div
              style="border-right:1px solid var(--tia-border);overflow:auto;background:var(--tia-panel)"
            >
              {#if instrOpen}
                <div class="section-label panel-head">
                  <span>Instructions</span>
                  <button
                    type="button"
                    class="collapse-btn"
                    title="Hide instructions"
                    onclick={() => (instrOpen = false)}>◀</button
                  >
                </div>
                <ElementPalette onAdd={onPaletteAdd} />
              {:else}
                <button
                  type="button"
                  class="expand-strip"
                  title="Show instructions"
                  onclick={() => (instrOpen = true)}>▶ Instructions</button
                >
              {/if}
            </div>
            <LadderEditor />
          </div>
        {:else if plc.view === "tags"}
          <SymbolTableView />
        {:else if plc.view === "memory"}
          <MemoryView />
        {:else if plc.view === "alloc"}
          <MemoryAllocationView />
        {:else if plc.view === "modbus"}
          <ModbusConfigView />
        {:else if plc.view === "math"}
          <MathHelpView />
        {:else if plc.view === "audit"}
          <div style="height:100%;background:#fff">
            <AuditPanel />
          </div>
        {:else if plc.view === "logs"}
          <LogsView />
        {/if}
      </div>
    </main>

    <aside class="tia-right">
      <WatchPanel />
    </aside>
  </div>

  <footer class="tia-statusbar">
    <span>Scan <strong>{plc.status?.scan_count ?? 0}</strong></span>
    <span>Last <strong>{plc.status?.last_scan_us ?? 0} µs</strong></span>
    <span>Cycle <strong>{plc.cycleMs} ms</strong></span>
    <span
      >Modbus <strong
        >{plc.modbus.running ? `ON :${plc.modbus.port}` : `OFF :${plc.modbus.port}`}</strong
      ></span
    >
    <span
      >Hash <strong title={plc.status?.program_hash}
        >{(plc.status?.program_hash ?? "").slice(0, 12) || "—"}</strong
      ></span
    >
    {#if (plc.status?.fault_code ?? 0) > 0}
      <span style="color:#ffaaaa"
        >Fault {plc.status?.fault_code}: {plc.status?.fault_message}</span
      >
    {/if}
  </footer>
</div>

{#if plc.dialogOpen && plc.editingElement}
  <ElementPropertiesDialog
    element={plc.editingElement}
    open={plc.dialogOpen}
    label={plc.labelFor(plc.editingElement.id)}
    onClose={() => plc.closeElementEditor()}
    onApply={(el, label) => plc.applyElementEdit(el, label)}
  />
{/if}
