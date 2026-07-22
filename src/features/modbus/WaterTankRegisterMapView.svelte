<script lang="ts">
  /**
   * Live SCADA register map for Water_Tank_Dual_Pump (HR 100…150).
   */
  import { plc } from "../../shared/stores/plc.svelte";
  import {
    createWaterTankModbusMap,
    waterTankRegisterTable,
    type WaterTankRegisterRow,
  } from "../../shared/lib/waterTankModbusMap";

  const rows = waterTankRegisterTable();
  let expandedHr = $state<number | null>(100);

  function packBits(
    getBit: (i: number) => boolean,
    start: number,
    count = 16
  ): number {
    let w = 0;
    for (let b = 0; b < count; b++) {
      if (getBit(start + b)) w |= 1 << b;
    }
    return w & 0xffff;
  }

  function liveValue(row: WaterTankRegisterRow): number | null {
    const m = plc.memory;
    const live = row.live;
    switch (live.kind) {
      case "holding":
        return m.holding_registers[live.plcR] ?? 0;
      case "pack_i":
        return packBits((i) => Boolean(m.discrete_inputs[i]), live.start);
      case "pack_q":
        return packBits((i) => Boolean(m.coils[i]), live.start);
      case "pack_m":
        return packBits((i) => Boolean(m.memory_bits?.[i]), live.start);
      case "reserved":
        return null;
    }
  }

  function hex16(n: number): string {
    return "0x" + (n & 0xffff).toString(16).toUpperCase().padStart(4, "0");
  }

  function bin16(n: number): string {
    return (n & 0xffff).toString(2).padStart(16, "0");
  }

  function toggleExpand(hr: number) {
    expandedHr = expandedHr === hr ? null : hr;
  }

  async function applyMap() {
    const err = await plc.saveModbusMap(createWaterTankModbusMap());
    plc.message = err
      ? `Modbus map error: ${err}`
      : "Water tank Modbus map (HR100–150) applied";
  }
</script>

<div class="page">
  <header class="hdr">
    <div>
      <h1>Mapa rejestrów Modbus — Water Tank</h1>
      <p>
        Wszystkie sygnały stacji w <strong>HR 100…150</strong> (FC03). Slave:
        <code>127.0.0.1:5020</code>. Wartości na żywo z obrazu procesu PLC.
      </p>
    </div>
    <div class="actions">
      <button type="button" class="tia-btn tia-btn-primary" onclick={() => applyMap()}>
        Zastosuj mapę do Modbus
      </button>
      <button type="button" class="tia-btn" onclick={() => plc.setView("modbus")}>
        Edytor reguł
      </button>
      <button type="button" class="tia-btn" onclick={() => plc.setView("ladder")}>
        ← LAD
      </button>
    </div>
  </header>

  <div class="legend">
    <span><i class="sw rw"></i> R/W — zapis SCADA (gdy Allow writes)</span>
    <span><i class="sw ro"></i> R — tylko odczyt (write-protect)</span>
    <span>HR100–103 = słowa bitowe · HR104–121 = proces · HR122–150 = rezerwa</span>
  </div>

  <div class="table-wrap">
    <table>
      <thead>
        <tr>
          <th>HR</th>
          <th>Symbol</th>
          <th>PLC</th>
          <th>R/W</th>
          <th>Live</th>
          <th>Hex</th>
          <th>Opis</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row (row.hr)}
          {@const val = liveValue(row)}
          <tr
            class:pack={!!row.bits}
            class:reserved={row.live.kind === "reserved"}
            class:open={expandedHr === row.hr}
          >
            <td class="hr">
              {#if row.bits}
                <button type="button" class="exp" onclick={() => toggleExpand(row.hr)}>
                  {expandedHr === row.hr ? "▼" : "▶"} {row.hr}
                </button>
              {:else}
                {row.hr}
              {/if}
            </td>
            <td class="sym">{row.symbol}</td>
            <td class="plc">{row.plc}</td>
            <td>
              <span class="badge" class:rw={row.access === "R/W"}>{row.access}</span>
            </td>
            <td class="live">
              {#if val == null}
                —
              {:else}
                <strong>{val}</strong>
              {/if}
            </td>
            <td class="hex mono">{val == null ? "—" : hex16(val)}</td>
            <td class="desc">{row.description}</td>
          </tr>
          {#if row.bits && expandedHr === row.hr}
            <tr class="bit-row">
              <td colspan="7">
                <div class="bits">
                  <div class="bin mono">
                    bits15…0:
                    <strong>{val == null ? "—" : bin16(val)}</strong>
                  </div>
                  <table class="bit-table">
                    <thead>
                      <tr>
                        <th>Bit</th>
                        <th>Tag</th>
                        <th>Stan</th>
                        <th>Znaczenie</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each row.bits as b}
                        {@const on = val != null && ((val >> b.bit) & 1) === 1}
                        <tr class:on>
                          <td>{b.bit}</td>
                          <td class="mono">{b.tag}</td>
                          <td><span class="bit" class:on>{on ? "1" : "0"}</span></td>
                          <td>{b.meaning}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              </td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .page {
    height: 100%;
    overflow: auto;
    padding: 12px 16px 32px;
    background: #f4f6f8;
  }
  .hdr {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 10px;
  }
  h1 {
    margin: 0 0 4px;
    font-size: 16px;
    color: #1a557d;
  }
  .hdr p {
    margin: 0;
    font-size: 12px;
    color: #5a6570;
    max-width: 640px;
    line-height: 1.4;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    font-size: 11px;
    color: #5a6570;
    margin-bottom: 10px;
    align-items: center;
  }
  .sw {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 2px;
    margin-right: 4px;
    vertical-align: middle;
  }
  .sw.rw {
    background: #2a88b8;
  }
  .sw.ro {
    background: #9aa3ab;
  }
  .table-wrap {
    background: #fff;
    border: 1px solid #c0c7ce;
    border-radius: 4px;
    overflow: auto;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  th {
    text-align: left;
    background: linear-gradient(180deg, #e8eef4, #d8e0e8);
    border-bottom: 1px solid #b0b8c0;
    padding: 7px 8px;
    font-size: 11px;
    font-weight: 700;
    color: #003d5c;
    position: sticky;
    top: 0;
    z-index: 1;
  }
  td {
    padding: 5px 8px;
    border-bottom: 1px solid #e8ecf0;
    vertical-align: top;
  }
  tr:hover td {
    background: #f5f9fc;
  }
  tr.reserved td {
    color: #9aa3ab;
    font-style: italic;
  }
  tr.pack td {
    background: #fafcff;
  }
  .hr {
    font-family: Consolas, monospace;
    font-weight: 800;
    color: #1a557d;
    white-space: nowrap;
  }
  .exp {
    border: none;
    background: transparent;
    font: inherit;
    font-weight: 800;
    color: #1a557d;
    cursor: pointer;
    padding: 0;
  }
  .sym {
    font-family: Consolas, monospace;
    font-weight: 700;
    color: #6b2d9b;
  }
  .plc {
    font-family: Consolas, monospace;
    font-size: 11px;
  }
  .live strong {
    font-family: Consolas, monospace;
    font-size: 13px;
    color: #0a6b38;
  }
  .hex {
    font-size: 11px;
    color: #5a6570;
  }
  .mono {
    font-family: Consolas, monospace;
  }
  .desc {
    color: #3a4550;
    line-height: 1.3;
    max-width: 360px;
  }
  .badge {
    display: inline-block;
    font-size: 10px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 2px;
    background: #e8ebef;
    color: #5a6570;
  }
  .badge.rw {
    background: #d6ebf7;
    color: #005f87;
  }
  .bit-row td {
    background: #f0f4f8 !important;
    padding: 8px 12px 12px;
  }
  .bin {
    font-size: 11px;
    margin-bottom: 6px;
    color: #3a4550;
  }
  .bit-table {
    width: auto;
    min-width: 420px;
    border: 1px solid #c5ccd2;
    background: #fff;
  }
  .bit-table th {
    position: static;
    font-size: 10px;
    padding: 4px 8px;
  }
  .bit-table td {
    padding: 3px 8px;
    font-size: 11px;
    border-bottom: 1px solid #eef1f4;
  }
  .bit-table tr.on td {
    background: #e8f8ef;
  }
  .bit {
    display: inline-block;
    min-width: 18px;
    text-align: center;
    font-family: Consolas, monospace;
    font-weight: 800;
    border-radius: 2px;
    background: #eef2f5;
    padding: 0 4px;
  }
  .bit.on {
    background: #00a651;
    color: #fff;
  }
</style>
