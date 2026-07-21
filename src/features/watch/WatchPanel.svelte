<script lang="ts">
  import { plc } from "../../shared/stores/plc.svelte";

  async function editHolding(addr: number) {
    const cur = plc.memory.holding_registers[addr] ?? 0;
    const raw = window.prompt(`R${addr}`, String(cur));
    if (raw == null) return;
    const v = Number(raw);
    if (!Number.isFinite(v) || v < 0 || v > 65535) return;
    await plc.setHolding(addr, Math.floor(v));
  }

  async function editMemoryWord(addr: number) {
    const cur = plc.memory.memory_words?.[addr] ?? 0;
    const raw = window.prompt(`MR${addr} (matrix-only on Modbus)`, String(cur));
    if (raw == null) return;
    const v = Number(raw);
    if (!Number.isFinite(v) || v < 0 || v > 65535) return;
    await plc.setMemoryWord(addr, Math.floor(v));
  }

  const iShow = $derived(Math.min(8, plc.memoryConfig.inputs));
  const qShow = $derived(Math.min(8, plc.memoryConfig.outputs));
  const mShow = $derived(Math.min(8, plc.memoryConfig.markers));
</script>

<div>
  <h2>Watch / force</h2>

  <div class="section-label">Inputs I (click = force)</div>
  <div class="watch-grid">
    {#each Array(iShow) as _, i}
      <button
        type="button"
        class="watch-bit"
        class:on={plc.memory.discrete_inputs[i]}
        onclick={() => plc.toggleDiscrete(i)}
      >
        <span class="tag">I{i}</span>
        <span class="val">{plc.memory.discrete_inputs[i] ? "1" : "0"}</span>
      </button>
    {/each}
  </div>

  <div class="section-label">Outputs Q</div>
  <div class="watch-grid">
    {#each Array(qShow) as _, i}
      <div class="watch-bit" class:on={plc.memory.coils[i]}>
        <span class="tag">Q{i}</span>
        <span class="val">{plc.memory.coils[i] ? "1" : "0"}</span>
      </div>
    {/each}
  </div>

  <div class="section-label">Holding R (click edit · demo R40–42)</div>
  {#each [0, 1, 40, 41, 42] as addr}
    <button type="button" class="watch-reg" onclick={() => editHolding(addr)}>
      <span>R{addr}</span>
      <span>{plc.memory.holding_registers[addr] ?? 0}</span>
    </button>
  {/each}

  <div class="section-label">Markers M (click = force)</div>
  <div class="watch-grid">
    {#each Array(mShow) as _, i}
      <button
        type="button"
        class="watch-bit"
        class:on={plc.memory.memory_bits?.[i]}
        onclick={() => plc.toggleMemoryBit(i)}
      >
        <span class="tag">M{i}</span>
        <span class="val">{plc.memory.memory_bits?.[i] ? "1" : "0"}</span>
      </button>
    {/each}
  </div>

  <div class="section-label">Internal MR (click edit)</div>
  {#each [0, 1, 2, 3] as addr}
    <button type="button" class="watch-reg" onclick={() => editMemoryWord(addr)}>
      <span>MR{addr}</span>
      <span>{plc.memory.memory_words?.[addr] ?? 0}</span>
    </button>
  {/each}

  <div class="section-label">Timers T (ET / Q)</div>
  {#each [0, 1, 2, 3] as ti}
    <div class="watch-reg" style="cursor:default">
      <span>T{ti}</span>
      <span
        >ET={plc.memory.timer_et?.[ti] ?? 0}
        Q={plc.memory.timer_q?.[ti] ? "1" : "0"}</span
      >
    </div>
  {/each}

  <div class="section-label">Counters C (CV / Q)</div>
  {#each [0, 1, 2, 10] as ci}
    <div class="watch-reg" style="cursor:default">
      <span>C{ci}</span>
      <span
        >CV={plc.memory.counter_cv?.[ci] ?? 0}
        Q={plc.memory.counter_q?.[ci] ? "1" : "0"}</span
      >
    </div>
  {/each}

  <div class="section-label">Diagnostics IW</div>
  <div style="padding:6px 10px;font-size:11px;font-family:var(--font-mono);color:var(--tia-muted)">
    IW0={plc.memory.input_registers?.[0] ?? 0}
    · IW1={plc.memory.input_registers?.[1] ?? 0}
    · IW4={plc.memory.input_registers?.[4] ?? 0}
  </div>
  <p style="padding:4px 10px;font-size:10px;color:var(--tia-muted);margin:0">
    Copy T/C → R: MOVE source <code>TV0</code> / <code>CV0</code>, dest <code>R10</code>.
  </p>

  <div class="section-label">Modbus</div>
  <div style="padding:6px 10px;font-size:11px;font-family:var(--font-mono);color:var(--tia-muted)">
    {#if plc.modbus.running}
      <span style="color:var(--tia-online);font-weight:700">ON</span> :{plc.modbus.port}
    {:else}
      <span>OFF</span> (port {plc.modbus.port})
    {/if}
  </div>
</div>
