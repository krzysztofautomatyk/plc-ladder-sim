<script lang="ts">
  /**
   * PLC tags table — TIA-style symbol table for bits & registers.
   */
  import { plc } from "../../shared/stores/plc.svelte";
  import type { DataType, MemArea, PlcSymbol } from "../../shared/lib/types";
  import { uid } from "../../shared/lib/demoProgram";

  let rows = $state<PlcSymbol[]>([]);

  $effect(() => {
    rows = plc.symbols.map((s) => ({ ...s }));
  });

  function displayFor(area: MemArea, index: number) {
    switch (area) {
      case "discrete":
        return `I${index}`;
      case "coil":
        return `Q${index}`;
      case "holding":
        return `MW${index}`;
      case "input_reg":
        return `IW${index}`;
      case "memory_bit":
        return `M${index}`;
      case "memory_word":
        return `MR${index}`;
    }
  }

  function addRow() {
    rows = [
      ...rows,
      {
        id: uid("tag"),
        name: "New_Tag",
        area: "discrete",
        index: 0,
        data_type: "bool",
        comment: "",
        address_display: "I0",
      },
    ];
  }

  function removeRow(id: string) {
    rows = rows.filter((r) => r.id !== id);
  }

  function onAreaChange(r: PlcSymbol, area: MemArea) {
    r.area = area;
    r.address_display = displayFor(area, r.index);
    r.data_type =
      area === "holding" || area === "input_reg" || area === "memory_word" ? "word" : "bool";
    rows = [...rows];
  }

  function onIndexChange(r: PlcSymbol, index: number) {
    r.index = Math.max(0, index);
    r.address_display = displayFor(r.area, r.index);
    rows = [...rows];
  }

  async function save() {
    const cleaned = rows.map((r) => ({
      ...r,
      address_display: displayFor(r.area, r.index),
      name: r.name.trim() || "Unnamed",
    }));
    await plc.saveSymbols(cleaned);
  }
</script>

<div class="lad-paper">
  <div class="tia-page-header">
    <div>
      <h1>PLC tags</h1>
      <p>Define named bits and registers (I / Q / MW / IW) used in ladder and Modbus export.</p>
    </div>
    <div class="tia-actions">
      <button type="button" class="tia-btn" onclick={addRow}>+ Tag</button>
      <button type="button" class="tia-btn tia-btn-primary" onclick={save}>Save table</button>
    </div>
  </div>

  <div class="tia-table-wrap">
    <table class="tia-table">
      <thead>
        <tr>
          <th style="width:18%">Name</th>
          <th style="width:12%">Data type</th>
          <th style="width:12%">Area</th>
          <th style="width:10%">Address</th>
          <th style="width:12%">Display</th>
          <th>Comment</th>
          <th style="width:60px"></th>
        </tr>
      </thead>
      <tbody>
        {#each rows as r (r.id)}
          <tr>
            <td><input bind:value={r.name} /></td>
            <td>
              <select
                value={r.data_type}
                onchange={(e) => {
                  r.data_type = e.currentTarget.value as DataType;
                  rows = [...rows];
                }}
              >
                <option value="bool">Bool</option>
                <option value="word">Word</option>
                <option value="int">Int</option>
                <option value="d_int">DInt</option>
              </select>
            </td>
            <td>
              <select
                value={r.area}
                onchange={(e) => onAreaChange(r, e.currentTarget.value as MemArea)}
              >
                <option value="discrete">I (Input)</option>
                <option value="coil">Q (Output)</option>
                <option value="holding">MW (Memory)</option>
                <option value="input_reg">IW (Input word)</option>
                <option value="memory_bit">M (Marker · internal)</option>
                <option value="memory_word">MR (Register · internal)</option>
              </select>
            </td>
            <td>
              <input
                type="number"
                min="0"
                value={r.index}
                onchange={(e) => onIndexChange(r, Number(e.currentTarget.value))}
              />
            </td>
            <td><code>{r.address_display || displayFor(r.area, r.index)}</code></td>
            <td><input bind:value={r.comment} /></td>
            <td>
              <button type="button" class="tia-btn" onclick={() => removeRow(r.id)}>×</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
