<script lang="ts">
  /**
   * Device configuration — Modbus TCP enable, port, address map visibility.
   */
  import { plc } from "../../shared/stores/plc.svelte";
  import type { MemArea, ModbusMapEntry, ModbusTable } from "../../shared/lib/types";
  import { uid } from "../../shared/lib/demoProgram";

  let port = $state(5020);
  let identity = $state(true);
  let entries = $state<ModbusMapEntry[]>([]);

  $effect(() => {
    port = plc.modbus.port;
    identity = plc.modbusMap.identity_fallback;
    entries = plc.modbusMap.entries.map((e) => ({ ...e }));
  });

  function addEntry() {
    entries = [
      ...entries,
      {
        id: uid("map"),
        enabled: true,
        symbol_name: "",
        plc_area: "coil",
        plc_index: 0,
        modbus_table: "coil",
        modbus_address: 0,
        comment: "",
      },
    ];
  }

  function removeEntry(id: string) {
    entries = entries.filter((e) => e.id !== id);
  }

  async function saveMap() {
    await plc.saveModbusMap({
      entries,
      identity_fallback: identity,
    });
  }

  async function applyPort() {
    await plc.setModbusPort(Number(port) || 5020);
  }

  const tableLabel = (t: ModbusTable) => {
    switch (t) {
      case "coil":
        return "0x Coils (FC01/05/15)";
      case "discrete":
        return "1x Discrete (FC02)";
      case "holding":
        return "4x Holding (FC03/06/16)";
      case "input_reg":
        return "3x Input (FC04)";
    }
  };
</script>

<div class="lad-paper">
  <div class="tia-page-header">
    <div>
      <h1>Modbus TCP — device configuration</h1>
      <p>
        Enable/disable slave, set listen port, and choose which PLC bits/registers are visible to
        SCADA.
      </p>
    </div>
  </div>

  <div style="padding:12px;display:grid;gap:12px;max-width:920px">
    <section
      style="border:1px solid var(--tia-border);background:#f8fafb;padding:12px;border-radius:2px"
    >
      <h3 style="margin:0 0 10px;font-size:13px;color:var(--tia-blue-dark)">Connection</h3>
      <div style="display:flex;flex-wrap:wrap;gap:12px;align-items:center">
        <label>
          Port
          <input type="number" min="1" max="65535" bind:value={port} style="width:90px;margin-left:6px" />
        </label>
        <button type="button" class="tia-btn" onclick={applyPort}>Apply port</button>
        <div class="tia-sep" style="width:1px;height:24px;background:var(--tia-border)"></div>
        <label title="Modbus is read-only until this is enabled explicitly.">
          <input
            type="checkbox"
            checked={plc.modbus.write_enabled}
            onchange={(e) => plc.setModbusWriteEnabled(e.currentTarget.checked)}
          />
          Allow SCADA writes
        </label>
        {#if plc.modbus.running}
          <span class="tia-badge modbus-on">LISTENING :{plc.modbus.port}</span>
          <button type="button" class="tia-btn tia-btn-stop" onclick={() => plc.stopModbus()}
            >Disable Modbus</button
          >
        {:else}
          <span class="tia-badge modbus-off">STOPPED</span>
          <button type="button" class="tia-btn tia-btn-run" onclick={() => plc.startModbus()}
            >Enable Modbus</button
          >
        {/if}
      </div>
      {#if plc.modbus.last_error}
        <p style="color:var(--tia-fault);margin:8px 0 0;font-size:12px">
          Error: {plc.modbus.last_error}
        </p>
      {/if}
      <p style="margin:10px 0 0;font-size:11px;color:var(--tia-muted)">
        Bind: {plc.modbus.bind || "127.0.0.1"} · local SCADA only by default · writes
        {plc.modbus.write_enabled ? "enabled" : "read-only"} · Unit ID any · default map identity
        when fallback enabled.
      </p>
    </section>

    <section>
      <div class="tia-page-header" style="border:1px solid var(--tia-border);border-bottom:none">
        <div>
          <h1 style="font-size:13px">Address map (visibility)</h1>
          <p>Only enabled rows are exposed. PLC address may differ from Modbus address.</p>
        </div>
        <div class="tia-actions">
          <label style="font-size:12px;display:flex;align-items:center;gap:6px">
            <input type="checkbox" bind:checked={identity} />
            Identity fallback (unmapped → same index)
          </label>
          <button type="button" class="tia-btn" onclick={addEntry}>+ Mapping</button>
          <button type="button" class="tia-btn tia-btn-primary" onclick={saveMap}>Save map</button>
        </div>
      </div>

      <div class="tia-table-wrap" style="padding-top:0">
        <table class="tia-table">
          <thead>
            <tr>
              <th style="width:50px">On</th>
              <th>Symbol</th>
              <th>PLC area</th>
              <th>PLC addr</th>
              <th>Modbus table</th>
              <th>Modbus addr</th>
              <th>Comment</th>
              <th style="width:50px"></th>
            </tr>
          </thead>
          <tbody>
            {#each entries as e (e.id)}
              <tr>
                <td style="text-align:center">
                  <input type="checkbox" bind:checked={e.enabled} />
                </td>
                <td><input bind:value={e.symbol_name} placeholder="optional" /></td>
                <td>
                  <select
                    value={e.plc_area}
                    onchange={(ev) => {
                      e.plc_area = ev.currentTarget.value as MemArea;
                      entries = [...entries];
                    }}
                  >
                    <option value="discrete">I</option>
                    <option value="coil">Q</option>
                    <option value="holding">MW</option>
                    <option value="input_reg">IW</option>
                  </select>
                </td>
                <td>
                  <input type="number" min="0" bind:value={e.plc_index} />
                </td>
                <td>
                  <select
                    value={e.modbus_table}
                    onchange={(ev) => {
                      e.modbus_table = ev.currentTarget.value as ModbusTable;
                      entries = [...entries];
                    }}
                  >
                    <option value="coil">{tableLabel("coil")}</option>
                    <option value="discrete">{tableLabel("discrete")}</option>
                    <option value="holding">{tableLabel("holding")}</option>
                    <option value="input_reg">{tableLabel("input_reg")}</option>
                  </select>
                </td>
                <td>
                  <input type="number" min="0" bind:value={e.modbus_address} />
                </td>
                <td><input bind:value={e.comment} /></td>
                <td>
                  <button type="button" class="tia-btn" onclick={() => removeEntry(e.id)}>×</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  </div>
</div>
