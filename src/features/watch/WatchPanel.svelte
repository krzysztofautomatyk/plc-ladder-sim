<script lang="ts">
  import { plc } from "../../shared/stores/plc.svelte";

  async function editHolding(addr: number) {
    const cur = plc.memory.holding_registers[addr] ?? 0;
    const raw = window.prompt(`MW${addr}`, String(cur));
    if (raw == null) return;
    const v = Number(raw);
    if (!Number.isFinite(v) || v < 0 || v > 65535) return;
    await plc.setHolding(addr, Math.floor(v));
  }

  async function editMemoryWord(addr: number) {
    const cur = plc.memory.memory_words?.[addr] ?? 0;
    const raw = window.prompt(`MR${addr} (internal, never on Modbus)`, String(cur));
    if (raw == null) return;
    const v = Number(raw);
    if (!Number.isFinite(v) || v < 0 || v > 65535) return;
    await plc.setMemoryWord(addr, Math.floor(v));
  }
</script>

<div>
  <h2>Watch / force</h2>

  <div class="section-label">Inputs I (click = force)</div>
  <div class="watch-grid">
    {#each Array(8) as _, i}
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
    {#each Array(8) as _, i}
      <div class="watch-bit" class:on={plc.memory.coils[i]}>
        <span class="tag">Q{i}</span>
        <span class="val">{plc.memory.coils[i] ? "1" : "0"}</span>
      </div>
    {/each}
  </div>

  <div class="section-label">Memory MW (click edit)</div>
  {#each [0, 1, 40, 41, 42] as addr}
    <button type="button" class="watch-reg" onclick={() => editHolding(addr)}>
      <span>MW{addr}</span>
      <span>{plc.memory.holding_registers[addr] ?? 0}</span>
    </button>
  {/each}

  <div class="section-label">Markers M (internal · click = force)</div>
  <div class="watch-grid">
    {#each Array(8) as _, i}
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

  <div class="section-label">Registers MR (internal · click edit)</div>
  {#each [0, 1, 2, 3] as addr}
    <button type="button" class="watch-reg" onclick={() => editMemoryWord(addr)}>
      <span>MR{addr}</span>
      <span>{plc.memory.memory_words?.[addr] ?? 0}</span>
    </button>
  {/each}

  <div class="section-label">Modbus</div>
  <div style="padding:6px 10px;font-size:11px;font-family:var(--font-mono);color:var(--tia-muted)">
    {#if plc.modbus.running}
      <span style="color:var(--tia-online);font-weight:700">ON</span> :{plc.modbus.port}
    {:else}
      <span>OFF</span> (port {plc.modbus.port})
    {/if}
  </div>
</div>
