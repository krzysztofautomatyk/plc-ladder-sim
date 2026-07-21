<script lang="ts">
  /**
   * Live process-image browser for all PLC memory areas.
   * Row counts follow Memory allocation; values come from the reactive snapshot
   * (compact UI image covers the first COMPACT_* cells — see backend memory.rs).
   */
  import { plc } from "../../shared/stores/plc.svelte";
  import type { MemArea, PlcSymbol } from "../../shared/lib/types";

  /** Cap rows per section so the page stays usable (scroll within allocation). */
  const MAX_SHOW_BITS = 64;
  const MAX_SHOW_WORDS = 64;

  function symbolFor(area: MemArea, index: number): PlcSymbol | undefined {
    return plc.symbols.find((s) => s.area === area && s.index === index);
  }

  function bitCount(allocated: number, available: number): number {
    const n = Math.min(allocated, MAX_SHOW_BITS, Math.max(0, available));
    return Math.max(0, n);
  }

  function wordCount(allocated: number, available: number): number {
    const n = Math.min(allocated, MAX_SHOW_WORDS, Math.max(0, available));
    return Math.max(0, n);
  }

  const mCount = $derived(
    bitCount(plc.memoryConfig.markers, plc.memory.memory_bits?.length ?? 0)
  );
  const qCount = $derived(bitCount(plc.memoryConfig.outputs, plc.memory.coils?.length ?? 0));
  const iCount = $derived(
    bitCount(plc.memoryConfig.inputs, plc.memory.discrete_inputs?.length ?? 0)
  );
  const rCount = $derived(
    wordCount(plc.memoryConfig.data16, plc.memory.holding_registers?.length ?? 0)
  );
  const mrCount = $derived(
    wordCount(plc.memoryConfig.internal16, plc.memory.memory_words?.length ?? 0)
  );

  const markers = $derived(Array.from({ length: mCount }, (_, i) => i));
  const inputs = $derived(Array.from({ length: iCount }, (_, i) => i));
  const outputs = $derived(Array.from({ length: qCount }, (_, i) => i));
  const registers = $derived(Array.from({ length: rCount }, (_, i) => i));
  const internalRegs = $derived(Array.from({ length: mrCount }, (_, i) => i));

  function commitWord(index: number, raw: string) {
    const v = Number(raw);
    if (!Number.isFinite(v) || v < 0 || v > 65535) return;
    plc.setHolding(index, Math.floor(v));
  }

  function commitMr(index: number, raw: string) {
    const v = Number(raw);
    if (!Number.isFinite(v) || v < 0 || v > 65535) return;
    plc.setMemoryWord(index, Math.floor(v));
  }

  function rangeNote(prefix: string, allocated: number, shown: number): string {
    if (allocated < 1) return "none allocated";
    const last = Math.max(0, allocated - 1);
    if (shown < allocated) {
      return `${prefix}0–${prefix}${last} allocated · showing 0–${shown - 1}`;
    }
    return `${prefix}0–${prefix}${last}`;
  }
</script>

<div class="mem-view">
  <div class="tia-page-header">
    <div>
      <h1>Process image</h1>
      <p>
        Live I / Q / M / R / MR. Ranges follow <strong>Memory allocation</strong>. M and MR appear on
        Modbus only via an explicit Translation Matrix rule (never identity fallback).
      </p>
    </div>
    <div class="tia-actions">
      <button type="button" class="tia-btn" onclick={() => plc.setView("alloc")}>Memory allocation</button>
      <button type="button" class="tia-btn" onclick={() => plc.setView("tags")}>PLC tags</button>
      <button type="button" class="tia-btn" onclick={() => plc.resetMemory()}>Reset I/O</button>
    </div>
  </div>

  <div class="mem-body">
    <section>
      <div class="sec-title">
        Inputs I <span>({rangeNote("I", plc.memoryConfig.inputs, iCount)} · click = force)</span>
      </div>
      <div class="tia-table-wrap">
        <table class="tia-table">
          <thead>
            <tr>
              <th class="c-addr">Address</th>
              <th class="c-name">Name</th>
              <th class="c-val">Value</th>
              <th>Comment</th>
            </tr>
          </thead>
          <tbody>
            {#each inputs as i (i)}
              {@const sym = symbolFor("discrete", i)}
              <tr>
                <td class="c-addr mono">I{i}</td>
                <td class="c-name">{sym?.name ?? ""}</td>
                <td class="c-val">
                  <button
                    type="button"
                    class="bit"
                    class:on={plc.memory.discrete_inputs?.[i]}
                    onclick={() => plc.toggleDiscrete(i)}
                    title="Click to force"
                  >
                    {plc.memory.discrete_inputs?.[i] ? "1" : "0"}
                  </button>
                </td>
                <td class="comment">{sym?.comment ?? ""}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>

    <section>
      <div class="sec-title">
        Outputs Q <span>({rangeNote("Q", plc.memoryConfig.outputs, qCount)})</span>
      </div>
      <div class="tia-table-wrap">
        <table class="tia-table">
          <thead>
            <tr>
              <th class="c-addr">Address</th>
              <th class="c-name">Name</th>
              <th class="c-val">Value</th>
              <th>Comment</th>
            </tr>
          </thead>
          <tbody>
            {#each outputs as i (i)}
              {@const sym = symbolFor("coil", i)}
              <tr>
                <td class="c-addr mono">Q{i}</td>
                <td class="c-name">{sym?.name ?? ""}</td>
                <td class="c-val">
                  <span class="bit" class:on={plc.memory.coils?.[i]}>
                    {plc.memory.coils?.[i] ? "1" : "0"}
                  </span>
                </td>
                <td class="comment">{sym?.comment ?? ""}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>

    <section>
      <div class="sec-title">
        Markers M <span>({rangeNote("M", plc.memoryConfig.markers, mCount)} · click = force)</span>
      </div>
      <div class="tia-table-wrap">
        <table class="tia-table">
          <thead>
            <tr>
              <th class="c-addr">Address</th>
              <th class="c-name">Name</th>
              <th class="c-val">Value</th>
              <th>Comment</th>
            </tr>
          </thead>
          <tbody>
            {#each markers as i (i)}
              {@const sym = symbolFor("memory_bit", i)}
              <tr>
                <td class="c-addr mono">M{i}</td>
                <td class="c-name">{sym?.name ?? ""}</td>
                <td class="c-val">
                  <button
                    type="button"
                    class="bit"
                    class:on={plc.memory.memory_bits?.[i]}
                    onclick={() => plc.toggleMemoryBit(i)}
                    title="Click to force"
                  >
                    {plc.memory.memory_bits?.[i] ? "1" : "0"}
                  </button>
                </td>
                <td class="comment">{sym?.comment ?? ""}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>

    <section>
      <div class="sec-title">
        Holding R <span>({rangeNote("R", plc.memoryConfig.data16, rCount)} · edit value)</span>
      </div>
      <div class="tia-table-wrap">
        <table class="tia-table">
          <thead>
            <tr>
              <th class="c-addr">Address</th>
              <th class="c-name">Name</th>
              <th class="c-val">Value</th>
              <th>Comment</th>
            </tr>
          </thead>
          <tbody>
            {#each registers as i (i)}
              {@const sym = symbolFor("holding", i)}
              <tr>
                <td class="c-addr mono">R{i}</td>
                <td class="c-name">{sym?.name ?? ""}</td>
                <td class="c-val">
                  <input
                    class="reg-input"
                    type="number"
                    min="0"
                    max="65535"
                    value={plc.memory.holding_registers?.[i] ?? 0}
                    onchange={(e) => commitWord(i, e.currentTarget.value)}
                  />
                </td>
                <td class="comment">{sym?.comment ?? ""}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="bank-note">
        Engine banks (not in this table head): timer ET/Q at R2048+, counter CV/Q at R3072+.
      </p>
    </section>

    <section>
      <div class="sec-title">
        Internal MR <span>({rangeNote("MR", plc.memoryConfig.internal16, mrCount)} · edit value)</span>
      </div>
      <div class="tia-table-wrap">
        <table class="tia-table">
          <thead>
            <tr>
              <th class="c-addr">Address</th>
              <th class="c-name">Name</th>
              <th class="c-val">Value</th>
              <th>Comment</th>
            </tr>
          </thead>
          <tbody>
            {#each internalRegs as i (i)}
              {@const sym = symbolFor("memory_word", i)}
              <tr>
                <td class="c-addr mono">MR{i}</td>
                <td class="c-name">{sym?.name ?? ""}</td>
                <td class="c-val">
                  <input
                    class="reg-input"
                    type="number"
                    min="0"
                    max="65535"
                    value={plc.memory.memory_words?.[i] ?? 0}
                    onchange={(e) => commitMr(i, e.currentTarget.value)}
                  />
                </td>
                <td class="comment">{sym?.comment ?? ""}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  </div>
</div>

<style>
  .mem-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--tia-paper);
  }
  .mem-body {
    flex: 1;
    overflow: auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    padding: 10px;
    align-items: start;
  }
  @media (max-width: 900px) {
    .mem-body {
      grid-template-columns: 1fr;
    }
  }
  .sec-title {
    font-size: 12px;
    font-weight: 700;
    color: var(--tia-blue-dark);
    padding: 4px 2px;
  }
  .sec-title span {
    font-weight: 400;
    color: var(--tia-muted);
  }
  .mono {
    font-family: var(--font-mono);
  }
  .c-addr {
    width: 68px;
    white-space: nowrap;
  }
  .c-name {
    width: 120px;
  }
  .c-val {
    width: 76px;
    text-align: center;
  }
  .comment {
    color: var(--tia-muted);
  }
  .bit {
    display: inline-block;
    width: 44px;
    padding: 2px 0;
    border: 1px solid var(--tia-border);
    border-radius: 3px;
    background: #eef2f5;
    font-family: var(--font-mono);
    font-weight: 700;
    cursor: default;
    text-align: center;
  }
  button.bit {
    cursor: pointer;
  }
  .bit.on {
    background: var(--tia-online);
    color: #fff;
    border-color: var(--tia-online);
  }
  .reg-input {
    width: 68px;
    text-align: right;
    font-family: var(--font-mono);
    border: 1px solid var(--tia-border);
    border-radius: 3px;
    padding: 1px 4px;
    background: #fff;
  }
  .bank-note {
    margin: 6px 2px 0;
    font-size: 10px;
    color: var(--tia-muted);
  }
</style>
