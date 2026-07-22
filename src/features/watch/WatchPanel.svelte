<script lang="ts">
  /**
   * Watch / force — process variables with 10-char label + full description.
   * Word force uses a modal dialog (window.prompt is unreliable in Tauri WebView
   * and live memory refresh would steal focus from inline inputs).
   */
  import { plc } from "../../shared/stores/plc.svelte";
  import type { MemArea, PlcSymbol } from "../../shared/lib/types";

  type WatchRow = {
    key: string;
    area: MemArea;
    index: number;
    addr: string;
    label: string;
    desc: string;
    kind: "bit" | "word";
    forceable: boolean;
  };

  type ForceDraft = {
    index: number;
    addr: string;
    label: string;
    desc: string;
    draft: string;
    error: string;
  };

  let forceDlg = $state<ForceDraft | null>(null);
  let forceInputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (forceDlg && forceInputEl) {
      // Focus after open so typing works immediately (no live refresh on draft)
      queueMicrotask(() => {
        forceInputEl?.focus();
        forceInputEl?.select();
      });
    }
  });

  function symFor(area: MemArea, index: number): PlcSymbol | undefined {
    return plc.symbols.find((s) => s.area === area && s.index === index);
  }

  function labelOf(area: MemArea, index: number, fallback: string): string {
    const s = symFor(area, index);
    const n = (s?.name ?? fallback).trim();
    return n.slice(0, 10) || fallback.slice(0, 10);
  }

  function descOf(area: MemArea, index: number, fallback = ""): string {
    return (symFor(area, index)?.comment ?? fallback).trim();
  }

  function bitVal(area: MemArea, index: number): boolean {
    const m = plc.memory;
    switch (area) {
      case "discrete":
        return Boolean(m.discrete_inputs[index]);
      case "coil":
        return Boolean(m.coils[index]);
      case "memory_bit":
        return Boolean(m.memory_bits?.[index]);
      default:
        return false;
    }
  }

  function wordVal(index: number): number {
    return plc.memory.holding_registers[index] ?? 0;
  }

  async function toggleBit(area: MemArea, index: number) {
    if (area === "discrete") await plc.toggleDiscrete(index);
    else if (area === "memory_bit") await plc.toggleMemoryBit(index);
  }

  /** Open force dialog — freezes draft so live refresh cannot overwrite typing. */
  function openForceWord(index: number, addr: string, label = "", desc = "") {
    forceDlg = {
      index,
      addr,
      label: label || addr,
      desc,
      draft: String(wordVal(index)),
      error: "",
    };
  }

  function closeForce() {
    forceDlg = null;
  }

  async function applyForce() {
    if (!forceDlg) return;
    const raw = forceDlg.draft.trim();
    const v = Number(raw);
    if (!Number.isFinite(v) || v < 0 || v > 65535 || !Number.isInteger(v)) {
      forceDlg = {
        ...forceDlg,
        error: "Enter an integer 0…65535",
      };
      return;
    }
    const idx = forceDlg.index;
    await plc.setHolding(idx, v);
    plc.message = `Forced ${forceDlg.addr} = ${v}`;
    forceDlg = null;
  }

  function onForceKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      void applyForce();
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeForce();
    }
  }

  /** Primary process variables for the wet-well (and generic fallbacks). */
  const processRows = $derived.by((): WatchRow[] => {
    const rows: WatchRow[] = [
      { key: "i0", area: "discrete", index: 0, addr: "I0", label: labelOf("discrete", 0, "SIM_EN"), desc: descOf("discrete", 0, "Enable simulation"), kind: "bit", forceable: true },
      { key: "i3", area: "discrete", index: 3, addr: "I3", label: labelOf("discrete", 3, "P1_FAULT"), desc: descOf("discrete", 3, "Pump 1 fault"), kind: "bit", forceable: true },
      { key: "i4", area: "discrete", index: 4, addr: "I4", label: labelOf("discrete", 4, "P2_FAULT"), desc: descOf("discrete", 4, "Pump 2 fault"), kind: "bit", forceable: true },
      { key: "i5", area: "discrete", index: 5, addr: "I5", label: labelOf("discrete", 5, "P1_LOCK"), desc: descOf("discrete", 5, "Lock pump 1"), kind: "bit", forceable: true },
      { key: "i6", area: "discrete", index: 6, addr: "I6", label: labelOf("discrete", 6, "P2_LOCK"), desc: descOf("discrete", 6, "Lock pump 2"), kind: "bit", forceable: true },
      { key: "i7", area: "discrete", index: 7, addr: "I7", label: labelOf("discrete", 7, "RST_STAT"), desc: descOf("discrete", 7, "Reset stats"), kind: "bit", forceable: true },
      { key: "i8", area: "discrete", index: 8, addr: "I8", label: labelOf("discrete", 8, "MAN_P1"), desc: descOf("discrete", 8, "Manual P1"), kind: "bit", forceable: true },
      { key: "i9", area: "discrete", index: 9, addr: "I9", label: labelOf("discrete", 9, "MAN_P2"), desc: descOf("discrete", 9, "Manual P2"), kind: "bit", forceable: true },

      { key: "q0", area: "coil", index: 0, addr: "Q0", label: labelOf("coil", 0, "P1_RUN"), desc: descOf("coil", 0, "Pump 1 run"), kind: "bit", forceable: false },
      { key: "q1", area: "coil", index: 1, addr: "Q1", label: labelOf("coil", 1, "P2_RUN"), desc: descOf("coil", 1, "Pump 2 run"), kind: "bit", forceable: false },
      { key: "q2", area: "coil", index: 2, addr: "Q2", label: labelOf("coil", 2, "ALM_HI"), desc: descOf("coil", 2, "High level"), kind: "bit", forceable: false },
      { key: "q3", area: "coil", index: 3, addr: "Q3", label: labelOf("coil", 3, "ALM_FLT"), desc: descOf("coil", 3, "Fault"), kind: "bit", forceable: false },
      { key: "q4", area: "coil", index: 4, addr: "Q4", label: labelOf("coil", 4, "ALM_FAIL"), desc: descOf("coil", 4, "Station fail"), kind: "bit", forceable: false },

      { key: "r100", area: "holding", index: 100, addr: "R100", label: labelOf("holding", 100, "LEVEL_cm"), desc: descOf("holding", 100, "Level [cm]"), kind: "word", forceable: true },
      { key: "r101", area: "holding", index: 101, addr: "R101", label: labelOf("holding", 101, "K_x100"), desc: descOf("holding", 101, "Inflow K×100"), kind: "word", forceable: true },
      { key: "r105", area: "holding", index: 105, addr: "R105", label: labelOf("holding", 105, "SP_STOP"), desc: descOf("holding", 105, "Stop 200 cm"), kind: "word", forceable: true },
      { key: "r106", area: "holding", index: 106, addr: "R106", label: labelOf("holding", 106, "SP_P1_ON"), desc: descOf("holding", 106, "P1 ON 700 cm"), kind: "word", forceable: true },
      { key: "r107", area: "holding", index: 107, addr: "R107", label: labelOf("holding", 107, "SP_P2_ON"), desc: descOf("holding", 107, "P2 ON 800 cm"), kind: "word", forceable: true },
      { key: "r120", area: "holding", index: 120, addr: "R120", label: labelOf("holding", 120, "P1_HH"), desc: descOf("holding", 120, "P1 hours"), kind: "word", forceable: false },
      { key: "r121", area: "holding", index: 121, addr: "R121", label: labelOf("holding", 121, "P1_MM"), desc: descOf("holding", 121, "P1 minutes"), kind: "word", forceable: false },
      { key: "r122", area: "holding", index: 122, addr: "R122", label: labelOf("holding", 122, "P1_SS"), desc: descOf("holding", 122, "P1 seconds"), kind: "word", forceable: false },
      { key: "r123", area: "holding", index: 123, addr: "R123", label: labelOf("holding", 123, "P2_HH"), desc: descOf("holding", 123, "P2 hours"), kind: "word", forceable: false },
      { key: "r124", area: "holding", index: 124, addr: "R124", label: labelOf("holding", 124, "P2_MM"), desc: descOf("holding", 124, "P2 minutes"), kind: "word", forceable: false },
      { key: "r125", area: "holding", index: 125, addr: "R125", label: labelOf("holding", 125, "P2_SS"), desc: descOf("holding", 125, "P2 seconds"), kind: "word", forceable: false },
      { key: "r126", area: "holding", index: 126, addr: "R126", label: labelOf("holding", 126, "P1_STARTS"), desc: descOf("holding", 126, "P1 starts"), kind: "word", forceable: false },
      { key: "r127", area: "holding", index: 127, addr: "R127", label: labelOf("holding", 127, "P2_STARTS"), desc: descOf("holding", 127, "P2 starts"), kind: "word", forceable: false },

      { key: "m2", area: "memory_bit", index: 2, addr: "M2", label: labelOf("memory_bit", 2, "DEMAND"), desc: descOf("memory_bit", 2, "Demand latch"), kind: "bit", forceable: false },
      { key: "m3", area: "memory_bit", index: 3, addr: "M3", label: labelOf("memory_bit", 3, "JOIN_P2"), desc: descOf("memory_bit", 3, "P2 latch ≥800→≤200"), kind: "bit", forceable: false },
    ];
    return rows;
  });

  const isWater = $derived(plc.program.name.includes("Water"));
  const liveNow = $derived(forceDlg ? wordVal(forceDlg.index) : 0);
</script>

<div class="watch">
  <h2>Watch / force</h2>
  <p class="sub">
    Kliknij wartość R → okno Force · bity I/M przełącz kliknięciem
    {#if isWater}
      · histereza <strong>200 / 700 / 800 cm</strong>
    {/if}
  </p>

  {#if isWater}
    <div class="section-label">Process (stacja pomp)</div>
    <div class="var-list">
      {#each processRows as row (row.key)}
        <div
          class="var-row"
          class:on={row.kind === "bit" && bitVal(row.area, row.index)}
          class:force={row.forceable}
          title={row.desc}
        >
          <div class="var-main">
            {#if row.kind === "bit" && row.forceable}
              <button
                type="button"
                class="bit-btn"
                class:on={bitVal(row.area, row.index)}
                onclick={() => toggleBit(row.area, row.index)}
              >
                {bitVal(row.area, row.index) ? "1" : "0"}
              </button>
            {:else if row.kind === "bit"}
              <span class="bit-ro" class:on={bitVal(row.area, row.index)}
                >{bitVal(row.area, row.index) ? "1" : "0"}</span
              >
            {:else if row.forceable}
              <button
                type="button"
                class="word-btn"
                title="Force value…"
                onclick={() => openForceWord(row.index, row.addr, row.label, row.desc)}
              >
                {wordVal(row.index)}
              </button>
            {:else}
              <span class="word-ro">{wordVal(row.index)}</span>
            {/if}
            <div class="meta">
              <div class="line1">
                <span class="addr">{row.addr}</span>
                <span class="label">{row.label}</span>
              </div>
              <div class="desc">{row.desc || "—"}</div>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="section-label">Inputs I</div>
    <div class="watch-grid">
      {#each Array(8) as _, i}
        <button
          type="button"
          class="watch-bit"
          class:on={plc.memory.discrete_inputs[i]}
          onclick={() => plc.toggleDiscrete(i)}
          title={descOf("discrete", i)}
        >
          <span class="tag">{labelOf("discrete", i, `I${i}`)}</span>
          <span class="val">{plc.memory.discrete_inputs[i] ? "1" : "0"}</span>
        </button>
      {/each}
    </div>
    <div class="section-label">Outputs Q</div>
    <div class="watch-grid">
      {#each Array(8) as _, i}
        <div class="watch-bit" class:on={plc.memory.coils[i]} title={descOf("coil", i)}>
          <span class="tag">{labelOf("coil", i, `Q${i}`)}</span>
          <span class="val">{plc.memory.coils[i] ? "1" : "0"}</span>
        </div>
      {/each}
    </div>
    <div class="section-label">Holding R</div>
    {#each [0, 1, 40, 41, 42, 100, 101, 105, 106, 107] as addr}
      <button
        type="button"
        class="watch-reg"
        onclick={() =>
          openForceWord(addr, `R${addr}`, labelOf("holding", addr, `R${addr}`), descOf("holding", addr))}
        title={descOf("holding", addr)}
      >
        <span>{labelOf("holding", addr, `R${addr}`)}</span>
        <span>{wordVal(addr)}</span>
      </button>
    {/each}
  {/if}

  <div class="section-label">Timers T0–T1</div>
  {#each [0, 1] as ti}
    <div class="watch-reg" style="cursor:default">
      <span>T{ti}</span>
      <span>ET={plc.memory.timer_et?.[ti] ?? 0} Q={plc.memory.timer_q?.[ti] ? "1" : "0"}</span>
    </div>
  {/each}

  <div class="section-label">Modbus</div>
  <div class="modbus-line">
    {#if plc.modbus.running}
      <span class="on">ON</span> :{plc.modbus.port}
    {:else}
      OFF :{plc.modbus.port}
    {/if}
  </div>
</div>

{#if forceDlg}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div
    class="force-backdrop"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeForce();
    }}
    onkeydown={onForceKey}
  >
    <div class="force-dlg" role="dialog" aria-modal="true" aria-labelledby="force-title">
      <header class="force-hdr">
        <h3 id="force-title">Force value</h3>
        <button type="button" class="force-x" onclick={closeForce} title="Close">×</button>
      </header>
      <div class="force-body">
        <div class="force-id">
          <span class="force-addr">{forceDlg.addr}</span>
          <span class="force-label">{forceDlg.label}</span>
        </div>
        {#if forceDlg.desc}
          <p class="force-desc">{forceDlg.desc}</p>
        {/if}
        <label class="force-field">
          <span>New value (0…65535)</span>
          <input
            class="force-input"
            type="text"
            inputmode="numeric"
            autocomplete="off"
            spellcheck="false"
            bind:this={forceInputEl}
            bind:value={forceDlg.draft}
            onkeydown={onForceKey}
          />
        </label>
        <p class="force-live">
          Live now: <strong>{liveNow}</strong>
          <span class="muted">(draft is frozen while you type)</span>
        </p>
        {#if forceDlg.addr === "R100"}
          <p class="force-warn">
            R100 LEVEL is written by the sim every scan while RUN + SIM_EN — force may be
            overwritten immediately. Use STOP or clear SIM_EN to hold a level.
          </p>
        {/if}
        {#if forceDlg.error}
          <p class="force-err">{forceDlg.error}</p>
        {/if}
      </div>
      <footer class="force-ftr">
        <button type="button" class="btn" onclick={closeForce}>Cancel</button>
        <button type="button" class="btn primary" onclick={() => applyForce()}>Apply</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .watch {
    font-size: 12px;
  }
  .sub {
    margin: 0 10px 8px;
    font-size: 10px;
    color: var(--tia-muted);
    line-height: 1.35;
  }
  .section-label {
    margin-top: 8px;
  }
  .var-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 6px 8px;
  }
  .var-row {
    border: 1px solid var(--tia-border-light, #dde3ea);
    border-radius: 3px;
    background: #f8fafb;
    padding: 4px 6px;
  }
  .var-row.on {
    border-color: #7dcea0;
    background: #eefaf3;
  }
  .var-main {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .meta {
    min-width: 0;
    flex: 1;
  }
  .line1 {
    display: flex;
    gap: 6px;
    align-items: baseline;
  }
  .addr {
    font-family: var(--font-mono, Consolas, monospace);
    font-weight: 700;
    color: var(--tia-blue-dark, #1a557d);
    font-size: 11px;
  }
  .label {
    font-family: var(--font-mono, Consolas, monospace);
    font-weight: 800;
    font-size: 11px;
    color: #003d5c;
    letter-spacing: 0.02em;
  }
  .desc {
    font-size: 10px;
    color: var(--tia-muted, #6a7682);
    line-height: 1.3;
    margin-top: 1px;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .bit-btn,
  .bit-ro {
    width: 36px;
    flex-shrink: 0;
    text-align: center;
    font-family: var(--font-mono, Consolas, monospace);
    font-weight: 800;
    border: 1px solid var(--tia-border, #c5ced6);
    border-radius: 3px;
    background: #eef2f5;
    padding: 4px 0;
  }
  .bit-btn {
    cursor: pointer;
  }
  .bit-btn.on,
  .bit-ro.on {
    background: var(--tia-online, #00a651);
    color: #fff;
    border-color: var(--tia-online, #00a651);
  }
  .word-btn,
  .word-ro {
    min-width: 48px;
    flex-shrink: 0;
    text-align: right;
    font-family: var(--font-mono, Consolas, monospace);
    font-weight: 700;
    border: 1px solid var(--tia-border, #c5ced6);
    border-radius: 3px;
    background: #fff;
    padding: 4px 6px;
  }
  .word-btn {
    cursor: pointer;
  }
  .word-btn:hover {
    border-color: var(--tia-blue, #0078a8);
    background: #e8f4fc;
  }
  .watch-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
    padding: 0 8px;
  }
  .watch-bit {
    display: flex;
    justify-content: space-between;
    gap: 6px;
    border: 1px solid var(--tia-border-light, #dde3ea);
    background: #fff;
    border-radius: 3px;
    padding: 4px 6px;
    font-size: 11px;
  }
  .watch-bit.on {
    background: #eefaf3;
    border-color: #7dcea0;
  }
  .watch-reg {
    display: flex;
    justify-content: space-between;
    width: calc(100% - 16px);
    margin: 2px 8px;
    border: 1px solid var(--tia-border-light, #dde3ea);
    background: #fff;
    border-radius: 3px;
    padding: 4px 8px;
    font-family: var(--font-mono, Consolas, monospace);
    font-size: 11px;
    cursor: pointer;
  }
  .modbus-line {
    padding: 6px 10px;
    font-size: 11px;
    font-family: var(--font-mono, Consolas, monospace);
    color: var(--tia-muted);
  }
  .modbus-line .on {
    color: var(--tia-online);
    font-weight: 700;
  }

  /* ── Force dialog ───────────────────────────────────────────────────── */
  .force-backdrop {
    position: fixed;
    inset: 0;
    z-index: 2000;
    background: rgba(20, 30, 40, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }
  .force-dlg {
    width: min(360px, 100%);
    background: #fff;
    border: 1px solid #9aa3ab;
    border-radius: 4px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.28);
    overflow: hidden;
  }
  .force-hdr {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: linear-gradient(180deg, #2a6f9e, #1a557d);
    color: #fff;
  }
  .force-hdr h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
  }
  .force-x {
    border: none;
    background: transparent;
    color: #fff;
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    padding: 0 4px;
  }
  .force-body {
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .force-id {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .force-addr {
    font-family: Consolas, monospace;
    font-weight: 800;
    font-size: 14px;
    color: #1a557d;
  }
  .force-label {
    font-family: Consolas, monospace;
    font-weight: 700;
    font-size: 12px;
    color: #6b2d9b;
  }
  .force-desc {
    margin: 0;
    font-size: 11px;
    color: #5a6570;
    line-height: 1.35;
  }
  .force-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
    font-weight: 600;
    color: #3a4550;
  }
  .force-input {
    font-family: Consolas, monospace;
    font-size: 18px;
    font-weight: 700;
    padding: 8px 10px;
    border: 2px solid #0078a8;
    border-radius: 3px;
    text-align: right;
  }
  .force-input:focus {
    outline: none;
    box-shadow: 0 0 0 3px rgba(0, 120, 168, 0.25);
  }
  .force-live {
    margin: 0;
    font-size: 11px;
    color: #3a4550;
  }
  .force-live .muted {
    color: #8a949e;
    margin-left: 4px;
  }
  .force-warn {
    margin: 0;
    font-size: 10px;
    line-height: 1.35;
    color: #8a5a00;
    background: #fff8e6;
    border: 1px solid #e6c87a;
    border-radius: 3px;
    padding: 6px 8px;
  }
  .force-err {
    margin: 0;
    font-size: 11px;
    color: #a00;
    font-weight: 600;
  }
  .force-ftr {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 8px 12px 12px;
    border-top: 1px solid #e0e6eb;
    background: #f4f7f9;
  }
  .btn {
    border: 1px solid #9aa3ab;
    background: linear-gradient(180deg, #fff, #eef1f4);
    font-size: 12px;
    font-weight: 600;
    padding: 5px 14px;
    border-radius: 3px;
    cursor: pointer;
  }
  .btn.primary {
    background: linear-gradient(180deg, #2a88b8, #005f87);
    border-color: #005f87;
    color: #fff;
  }
  .btn.primary:hover {
    filter: brightness(1.05);
  }
</style>
