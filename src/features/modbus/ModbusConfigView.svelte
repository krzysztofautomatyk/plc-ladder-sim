<script lang="ts">
  /**
   * Enterprise Modbus Translation Matrix — connection + rule editor.
   * Mapping types: Direct · Bit→Register · Register→Bit
   * Write protect: per-rule flag + global Strict / Silent Drop mode.
   */
  import { plc } from "../../shared/stores/plc.svelte";
  import type {
    MappingType,
    MemArea,
    ModbusMapEntry,
    ModbusTable,
    WriteProtectMode,
  } from "../../shared/lib/types";
  import { uid } from "../../shared/lib/demoProgram";

  let port = $state(5020);
  let identity = $state(true);
  let writeMode = $state<WriteProtectMode>("strict");
  let entries = $state<ModbusMapEntry[]>([]);
  let saveError = $state<string | null>(null);

  $effect(() => {
    port = plc.modbus.port;
    identity = plc.modbusMap.identity_fallback;
    writeMode = plc.modbusMap.write_protect_mode ?? "strict";
    entries = plc.modbusMap.entries.map((e) => normalizeEntry(e));
  });

  function normalizeEntry(e: ModbusMapEntry): ModbusMapEntry {
    return {
      id: e.id,
      enabled: e.enabled,
      mapping_type: e.mapping_type ?? "direct",
      symbol_name: e.symbol_name ?? "",
      plc_area: e.plc_area,
      plc_start: e.plc_start ?? 0,
      plc_bit_offset: e.plc_bit_offset ?? 0,
      modbus_table: e.modbus_table,
      modbus_start: e.modbus_start ?? 0,
      length: e.length ?? defaultLength(e.mapping_type ?? "direct"),
      is_write_protected: e.is_write_protected ?? false,
      comment: e.comment ?? "",
    };
  }

  function defaultLength(t: MappingType): number {
    switch (t) {
      case "register_to_bit":
        return 16;
      case "bit_to_register":
        return 1;
      default:
        return 1;
    }
  }

  function addEntry() {
    entries = [
      ...entries,
      {
        id: uid("map"),
        enabled: true,
        mapping_type: "direct",
        symbol_name: "",
        plc_area: "coil",
        plc_start: 0,
        plc_bit_offset: 0,
        modbus_table: "coil",
        modbus_start: 0,
        length: 1,
        is_write_protected: false,
        comment: "",
      },
    ];
    saveError = null;
  }

  function removeEntry(id: string) {
    entries = entries.filter((e) => e.id !== id);
  }

  function onMappingTypeChange(e: ModbusMapEntry, t: MappingType) {
    e.mapping_type = t;
    e.length = defaultLength(t);
    // Sensible defaults for PLC / Modbus tables per type.
    if (t === "bit_to_register") {
      if (e.plc_area === "holding" || e.plc_area === "input_reg" || e.plc_area === "memory_word") {
        e.plc_area = "memory_bit";
      }
      if (e.modbus_table === "coil" || e.modbus_table === "discrete") {
        e.modbus_table = "holding";
      }
    } else if (t === "register_to_bit") {
      if (e.plc_area === "coil" || e.plc_area === "discrete" || e.plc_area === "memory_bit") {
        e.plc_area = "holding";
      }
      if (e.modbus_table === "holding" || e.modbus_table === "input_reg") {
        e.modbus_table = "coil";
      }
    }
    entries = [...entries];
  }

  async function saveMap() {
    saveError = null;
    const err = await plc.saveModbusMap({
      entries: entries.map((e) => normalizeEntry(e)),
      identity_fallback: identity,
      write_protect_mode: writeMode,
    });
    if (err) saveError = err;
  }

  async function applyPort() {
    await plc.setModbusPort(Number(port) || 5020);
  }

  const tableLabel = (t: ModbusTable) => {
    switch (t) {
      case "coil":
        return "0x Coils";
      case "discrete":
        return "1x Discrete";
      case "holding":
        return "4x Holding";
      case "input_reg":
        return "3x Input";
    }
  };

  const typeLabel = (t: MappingType) => {
    switch (t) {
      case "direct":
        return "Direct";
      case "bit_to_register":
        return "Bit→Reg";
      case "register_to_bit":
        return "Reg→Bit";
    }
  };

  const areaLabel = (a: MemArea) => {
    switch (a) {
      case "discrete":
        return "I";
      case "coil":
        return "Q";
      case "holding":
        return "R";
      case "input_reg":
        return "IW";
      case "memory_bit":
        return "M";
      case "memory_word":
        return "MR";
    }
  };

  function coverageHint(e: ModbusMapEntry): string {
    const t = e.mapping_type ?? "direct";
    if (t === "bit_to_register") {
      return `${areaLabel(e.plc_area)}${e.plc_start}…${e.plc_start + 15} → HR/IR ${e.modbus_start}`;
    }
    if (t === "register_to_bit") {
      return `${areaLabel(e.plc_area)}${e.plc_start} → ${tableLabel(e.modbus_table)} ${e.modbus_start}…${e.modbus_start + 15}`;
    }
    return `${areaLabel(e.plc_area)}${e.plc_start} ↔ ${tableLabel(e.modbus_table)} ${e.modbus_start}`;
  }
</script>

<div class="lad-paper">
  <div class="tia-page-header">
    <div>
      <h1>Modbus TCP — Translation Matrix</h1>
      <p>
        Enterprise mapping engine: Direct, Bit→Register, Register→Bit rules with per-address write
        protection. SCADA writes require both the global allow flag and a non-protected rule.
      </p>
    </div>
  </div>

  <div style="padding:12px;display:grid;gap:12px;max-width:1200px">
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
        <label title="When a rule is write-protected: Strict returns exception 0x02; Silent Drop ACKs without mutating PLC.">
          Write-protect mode
          <select
            bind:value={writeMode}
            style="margin-left:6px"
            title="Applies when is_write_protected is true"
          >
            <option value="strict">Strict (exception)</option>
            <option value="silent_drop">Silent drop</option>
          </select>
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
        {plc.modbus.write_enabled ? "enabled" : "read-only"} · M/MR only via explicit rules (never
        identity fallback) · Unit ID any.
      </p>
      {#if writeMode === "silent_drop"}
        <p style="margin:8px 0 0;font-size:11px;color:var(--tia-fault)">
          Warning: Silent drop ACKs protected writes without changing PLC memory — SCADA may show
          success while the process image is unchanged.
        </p>
      {/if}
    </section>

    <section>
      <div class="tia-page-header" style="border:1px solid var(--tia-border);border-bottom:none">
        <div>
          <h1 style="font-size:13px">Address map (translation matrix)</h1>
          <p>
            Enabled rows are compiled into an O(1) index. Overlapping Modbus addresses are rejected
            on save.
          </p>
        </div>
        <div class="tia-actions">
          <label style="font-size:12px;display:flex;align-items:center;gap:6px">
            <input type="checkbox" bind:checked={identity} />
            Identity fallback (unmapped → same index; never M/MR)
          </label>
          <button type="button" class="tia-btn" onclick={addEntry}>+ Rule</button>
          <button type="button" class="tia-btn tia-btn-primary" onclick={saveMap}>Save map</button>
        </div>
      </div>

      {#if saveError}
        <p
          style="margin:0;padding:8px 12px;background:#fdecea;color:var(--tia-fault);font-size:12px;border:1px solid var(--tia-border);border-top:none"
        >
          Validation: {saveError}
        </p>
      {/if}

      <div class="tia-table-wrap" style="padding-top:0;overflow-x:auto">
        <table class="tia-table">
          <thead>
            <tr>
              <th style="width:40px">On</th>
              <th>Type</th>
              <th>PLC area</th>
              <th>PLC start</th>
              <th title="Bit offset inside PLC word (RegisterToBit)">Bit off</th>
              <th>Modbus table</th>
              <th>MB start</th>
              <th title="Modbus span (Direct multi-point length)">Len</th>
              <th title="Block Modbus writes to this rule">WProt</th>
              <th>Symbol</th>
              <th>Description</th>
              <th style="width:50px"></th>
            </tr>
          </thead>
          <tbody>
            {#each entries as e (e.id)}
              <tr>
                <td style="text-align:center">
                  <input type="checkbox" bind:checked={e.enabled} />
                </td>
                <td>
                  <select
                    value={e.mapping_type ?? "direct"}
                    onchange={(ev) =>
                      onMappingTypeChange(e, ev.currentTarget.value as MappingType)}
                    title={coverageHint(e)}
                  >
                    <option value="direct">{typeLabel("direct")}</option>
                    <option value="bit_to_register">{typeLabel("bit_to_register")}</option>
                    <option value="register_to_bit">{typeLabel("register_to_bit")}</option>
                  </select>
                </td>
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
                    <option value="memory_bit">M</option>
                    <option value="holding">R</option>
                    <option value="memory_word">MR</option>
                    <option value="input_reg">IW</option>
                  </select>
                </td>
                <td>
                  <input type="number" min="0" bind:value={e.plc_start} style="width:72px" />
                </td>
                <td>
                  <input
                    type="number"
                    min="0"
                    max="15"
                    bind:value={e.plc_bit_offset}
                    style="width:52px"
                    disabled={(e.mapping_type ?? "direct") !== "register_to_bit"}
                    title="Only for Register→Bit"
                  />
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
                  <input type="number" min="0" bind:value={e.modbus_start} style="width:72px" />
                </td>
                <td>
                  <input
                    type="number"
                    min="1"
                    bind:value={e.length}
                    style="width:52px"
                    disabled={(e.mapping_type ?? "direct") !== "direct"}
                    title="Direct multi-point span; fixed for Bit→Reg / Reg→Bit"
                  />
                </td>
                <td style="text-align:center">
                  <input type="checkbox" bind:checked={e.is_write_protected} />
                </td>
                <td><input bind:value={e.symbol_name} placeholder="optional" style="min-width:80px" /></td>
                <td><input bind:value={e.comment} style="min-width:120px" /></td>
                <td>
                  <button type="button" class="tia-btn" onclick={() => removeEntry(e.id)}>×</button>
                </td>
              </tr>
              <tr class="hint-row">
                <td colspan="12" style="font-size:10px;color:var(--tia-muted);padding:2px 8px 8px">
                  {coverageHint(e)}
                  {#if e.is_write_protected}
                    · write-protected ({writeMode === "silent_drop" ? "silent drop" : "strict"})
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  </div>
</div>
