<script lang="ts">
  /**
   * Memory allocation — size the PLC address spaces (like a classic PLC
   * "Memory Allocation" editor). Each area has a configurable element count,
   * bounded by the physical process-image pool; the resulting address range is
   * shown live. 16- and 32-bit data registers share one R pool (each 32-bit
   * register uses two words). Validated + persisted via the backend.
   */
  import { plc } from "../../shared/stores/plc.svelte";
  import type { MemoryConfig } from "../../shared/lib/types";

  type FieldKey = keyof MemoryConfig;
  type Kind = "bit" | "word" | "dword" | "instance";

  interface Row {
    key: FieldKey;
    label: string;
    prefix: string;
    kind: Kind;
    internal: boolean;
    note: string;
  }

  const ROWS: Row[] = [
    { key: "inputs", label: "Inputs", prefix: "I", kind: "bit", internal: false, note: "Discrete inputs (Modbus 1x)" },
    { key: "outputs", label: "Outputs", prefix: "Q", kind: "bit", internal: false, note: "Coils / outputs (Modbus 0x)" },
    { key: "markers", label: "Markers", prefix: "M", kind: "bit", internal: true, note: "Internal marker bits" },
    { key: "data16", label: "Data registers 16-bit", prefix: "R", kind: "word", internal: false, note: "Holding registers (Modbus 4x)" },
    { key: "data32", label: "Data registers 32-bit", prefix: "RD", kind: "dword", internal: false, note: "Two R words each (overlay top of R pool)" },
    { key: "internal16", label: "Internal registers", prefix: "MR", kind: "word", internal: true, note: "Internal 16-bit registers" },
    { key: "timers", label: "Timers", prefix: "T", kind: "instance", internal: true, note: "Timer instances (TON/TOF/RTO)" },
    { key: "counters", label: "Counters", prefix: "C", kind: "instance", internal: true, note: "Counter instances (CTU/CTD)" },
  ];

  const DEFAULTS: MemoryConfig = {
    inputs: 128,
    outputs: 128,
    markers: 1024,
    data16: 1024,
    data32: 0,
    internal16: 1024,
    timers: 64,
    counters: 64,
  };

  let cfg = $state<MemoryConfig>({ ...plc.memoryConfig });

  const limits = $derived(plc.memoryLimits);
  const poolWords = $derived(cfg.data16 + 2 * cfg.data32);
  const poolOver = $derived(poolWords > plc.registerPool);
  const poolPct = $derived(Math.min(100, Math.round((poolWords / plc.registerPool) * 100)));

  function overMax(key: FieldKey): boolean {
    return cfg[key] > limits[key] || cfg[key] < 0;
  }
  const anyError = $derived(ROWS.some((r) => overMax(r.key)) || poolOver);

  function rangeText(r: Row): string {
    const n = cfg[r.key];
    if (!n || n < 1) return "— (none allocated)";
    if (r.kind === "dword") {
      const base = cfg.data16;
      const lastWord = base + 2 * n - 1;
      return `RD0–RD${n - 1}   ·   words R${base}–R${lastWord}`;
    }
    return `${r.prefix}0–${r.prefix}${n - 1}`;
  }

  function reset() {
    cfg = { ...DEFAULTS };
  }

  function reload() {
    cfg = { ...plc.memoryConfig };
  }

  async function save() {
    if (anyError) return;
    await plc.saveMemoryConfig({ ...cfg });
    cfg = { ...plc.memoryConfig };
  }

  const dirty = $derived(ROWS.some((r) => cfg[r.key] !== plc.memoryConfig[r.key]));
</script>

<div class="alloc-view">
  <div class="tia-page-header">
    <div>
      <h1>Memory allocation</h1>
      <p>Size each address space; the range updates live. 16/32-bit registers share the R pool.</p>
    </div>
    <div class="tia-actions">
      <button type="button" class="tia-btn" onclick={reset}>Reset to default</button>
      <button type="button" class="tia-btn" onclick={reload} disabled={!dirty}>Revert</button>
      <button type="button" class="tia-btn tia-btn-primary" onclick={save} disabled={anyError || !dirty}>
        Save
      </button>
    </div>
  </div>

  <div class="alloc-body">
    <section class="alloc-table-wrap">
      <table class="tia-table alloc-table">
        <thead>
          <tr>
            <th>Area</th>
            <th class="c-count">Count</th>
            <th class="c-max">Max</th>
            <th>Range</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          {#each ROWS as r (r.key)}
            <tr class:err={overMax(r.key)}>
              <td>
                <span class="prefix">{r.prefix}</span>
                {r.label}
                {#if r.internal}<span class="badge-int" title="Never on Modbus">internal</span>{/if}
              </td>
              <td class="c-count">
                <input
                  type="number"
                  min="0"
                  max={limits[r.key]}
                  bind:value={cfg[r.key]}
                />
              </td>
              <td class="c-max mono">{limits[r.key]}</td>
              <td class="mono range">{rangeText(r)}</td>
              <td class="desc">{r.note}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      <div class="pool" class:err={poolOver}>
        <div class="pool-head">
          <strong>R register pool</strong>
          <span class="mono">{poolWords} / {plc.registerPool} words ({poolPct}%)</span>
        </div>
        <div class="bar"><div class="fill" style="width:{poolPct}%"></div></div>
        {#if poolOver}
          <div class="pool-msg">
            16-bit + 32-bit registers exceed the {plc.registerPool}-word R pool. Reduce
            <span class="mono">R</span> or <span class="mono">RD</span>.
          </div>
        {/if}
      </div>

      {#if plc.message}
        <p class="save-msg">{plc.message}</p>
      {/if}
    </section>

    <aside class="alloc-help">
      <h2>Help</h2>
      <p>
        Allocate how many elements of each area the program may use. The <em>Range</em> column
        shows the valid addresses; addresses outside the allocation are rejected in the ladder
        editor.
      </p>
      <dl>
        <dt><span class="prefix">I</span> Inputs</dt>
        <dd>Discrete inputs <span class="mono">I0…</span>. Readable on Modbus (1x / FC02).</dd>
        <dt><span class="prefix">Q</span> Outputs</dt>
        <dd>Coils <span class="mono">Q0…</span>. Read/write on Modbus (0x / FC01,05,15).</dd>
        <dt><span class="prefix">M</span> Markers <span class="badge-int">internal</span></dt>
        <dd>Working bits <span class="mono">M0…</span> for interlocks, sequence steps. Never on Modbus.</dd>
        <dt><span class="prefix">R</span> Data registers 16-bit</dt>
        <dd>Holding words <span class="mono">R0…</span>. Read/write on Modbus (4x / FC03,06,16).</dd>
        <dt><span class="prefix">RD</span> Data registers 32-bit</dt>
        <dd>
          Each <span class="mono">RD</span> uses <strong>two consecutive R words</strong>, allocated
          above the 16-bit range. Example: <span class="mono">R0–R999</span> as 16-bit +
          <span class="mono">RD0–RD499</span> occupying <span class="mono">R1000–R1999</span>.
        </dd>
        <dt><span class="prefix">MR</span> Internal registers <span class="badge-int">internal</span></dt>
        <dd>Internal words <span class="mono">MR0…</span> for scratch math / setpoints. Never on Modbus.</dd>
        <dt><span class="prefix">T</span> Timers</dt>
        <dd>Timer instances <span class="mono">T0…</span> (TON / TOF / RTO).</dd>
        <dt><span class="prefix">C</span> Counters</dt>
        <dd>Counter instances <span class="mono">C0…</span> (CTU / CTD).</dd>
      </dl>
      <p class="tip">
        The <strong>R pool</strong> is shared by 16- and 32-bit registers:
        <span class="mono">data16 + 2 × data32 ≤ {plc.registerPool}</span>. Counts are capped at the
        physical process-image sizes shown in the <em>Max</em> column.
      </p>
    </aside>
  </div>
</div>

<style>
  .alloc-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--tia-paper);
  }
  .alloc-body {
    flex: 1;
    overflow: auto;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 14px;
    padding: 12px;
    align-items: start;
  }
  @media (max-width: 1000px) {
    .alloc-body {
      grid-template-columns: 1fr;
    }
  }
  .alloc-table {
    font-size: 12px;
  }
  .alloc-table td,
  .alloc-table th {
    vertical-align: middle;
  }
  .alloc-table tr.err td {
    background: #fdecec;
  }
  .c-count {
    width: 96px;
  }
  .c-max {
    width: 60px;
    text-align: right;
    color: var(--tia-muted);
  }
  .alloc-table input[type="number"] {
    width: 84px;
    text-align: right;
    font-family: var(--font-mono);
    border: 1px solid var(--tia-border);
    border-radius: 3px;
    padding: 2px 4px;
    background: #fff;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .range {
    color: var(--tia-blue-dark);
  }
  .desc {
    color: var(--tia-muted);
  }
  .prefix {
    display: inline-block;
    min-width: 22px;
    padding: 0 4px;
    margin-right: 4px;
    border-radius: 3px;
    background: var(--tia-blue);
    color: #fff;
    font-family: var(--font-mono);
    font-weight: 700;
    text-align: center;
  }
  .badge-int {
    margin-left: 6px;
    padding: 0 5px;
    border-radius: 8px;
    background: #eef2f5;
    border: 1px solid var(--tia-border-light);
    color: var(--tia-muted);
    font-size: 10px;
  }
  .pool {
    margin-top: 12px;
    padding: 10px 12px;
    border: 1px solid var(--tia-border-light);
    border-radius: 4px;
    background: #f8fafb;
  }
  .pool.err {
    border-color: var(--tia-fault);
    background: #fdecec;
  }
  .pool-head {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    margin-bottom: 6px;
  }
  .bar {
    height: 8px;
    background: #dde4ea;
    border-radius: 4px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--tia-online);
  }
  .pool.err .fill {
    background: var(--tia-fault);
  }
  .pool-msg {
    margin-top: 6px;
    font-size: 11px;
    color: var(--tia-fault);
  }
  .save-msg {
    margin-top: 10px;
    font-size: 11px;
    color: var(--tia-muted);
  }
  .alloc-help {
    border: 1px solid var(--tia-border-light);
    border-radius: 4px;
    background: #f8fafb;
    padding: 10px 12px;
    font-size: 12px;
  }
  .alloc-help h2 {
    margin: 0 0 6px;
    font-size: 13px;
    color: var(--tia-blue-dark);
  }
  .alloc-help dl {
    margin: 8px 0;
  }
  .alloc-help dt {
    margin-top: 8px;
    font-weight: 600;
  }
  .alloc-help dd {
    margin: 2px 0 0;
    color: var(--tia-muted);
  }
  .alloc-help .tip {
    margin-top: 10px;
    padding-top: 8px;
    border-top: 1px solid var(--tia-border-light);
    color: var(--tia-text);
  }
</style>
