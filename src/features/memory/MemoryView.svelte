<script lang="ts">
  /**
   * Memory (M / MR) — configuration & live view for the internal, ladder-only
   * memory areas that are never exposed on Modbus TCP.
   *
   *  - M<n>  : internal marker bits   (M0–M4095)   — click a value to force it
   *  - MR<n> : internal registers     (MR0–MR1023) — edit the value inline
   *
   * Names and comments come from the PLC tags table (area = memory_bit /
   * memory_word); live values come from the reactive process-image snapshot.
   */
  import { plc } from "../../shared/stores/plc.svelte";
  import type { MemArea, PlcSymbol } from "../../shared/lib/types";

  const M_ROWS = 32;
  const MR_ROWS = 32;

  function symbolFor(area: MemArea, index: number): PlcSymbol | undefined {
    return plc.symbols.find((s) => s.area === area && s.index === index);
  }

  function commitWord(index: number, raw: string) {
    const v = Number(raw);
    if (!Number.isFinite(v) || v < 0 || v > 65535) return;
    plc.setMemoryWord(index, Math.floor(v));
  }

  const markers = $derived(Array.from({ length: M_ROWS }, (_, i) => i));
  const registers = $derived(Array.from({ length: MR_ROWS }, (_, i) => i));
</script>

<div class="mem-view">
  <div class="tia-page-header">
    <div>
      <h1>Memory (M / MR)</h1>
      <p>Internal marker bits and registers — usable in ladder logic, <strong>never on Modbus</strong>.</p>
    </div>
    <div class="tia-actions">
      <button type="button" class="tia-btn" onclick={() => plc.setView("tags")}>Edit names in PLC tags</button>
      <button type="button" class="tia-btn" onclick={() => plc.resetMemory()}>Reset I/O</button>
    </div>
  </div>

  <div class="mem-body">
    <section>
      <div class="sec-title">Markers — M0…M{M_ROWS - 1} <span>(range M0–M4095 · click value = force)</span></div>
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
      <div class="sec-title">Registers — MR0…MR{MR_ROWS - 1} <span>(range MR0–MR1023 · edit value)</span></div>
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
                    onchange={(e) => commitWord(i, e.currentTarget.value)}
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
    width: 44px;
    padding: 2px 0;
    border: 1px solid var(--tia-border);
    border-radius: 3px;
    background: #eef2f5;
    font-family: var(--font-mono);
    font-weight: 700;
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
</style>
