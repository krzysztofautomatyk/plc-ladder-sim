<script lang="ts">
  /**
   * Diagnostics — live application log viewer (TIA-style, matches Audit trail).
   *
   * Reads the in-memory tracing ring buffer via `get_logs`, with a level filter,
   * auto-refresh, freeze-scroll, clear and copy. This is where Modbus TCP activity
   * (client connect / request / exception / disconnect) and bind errors surface,
   * so operators can diagnose connectivity without an attached console.
   */
  import { onMount } from "svelte";
  import { plc } from "../../shared/stores/plc.svelte";

  const levels = ["trace", "debug", "info", "warn", "error"] as const;

  let level = $state<string>("info");
  let auto = $state(true);
  let paused = $state(false);
  let scroller = $state<HTMLDivElement>();

  async function refresh() {
    await plc.refreshLogs(1000, level);
    if (!paused) {
      queueMicrotask(() => {
        if (scroller) scroller.scrollTop = scroller.scrollHeight;
      });
    }
  }

  async function clearLogs() {
    await plc.clearLogs();
  }

  async function copyAll() {
    const text = plc.logs
      .map((l) =>
        `${l.ts} ${l.level.toUpperCase().padEnd(5)} ${l.target} ${l.message} ${l.fields}`.trimEnd()
      )
      .join("\n");
    try {
      await navigator.clipboard.writeText(text);
      plc.message = `Copied ${plc.logs.length} log lines`;
    } catch {
      plc.message = "Clipboard unavailable";
    }
  }

  onMount(() => {
    refresh();
    const id = setInterval(() => {
      if (auto) refresh();
    }, 1000);
    return () => clearInterval(id);
  });
</script>

<div class="logs-view">
  <div class="tia-page-header">
    <div>
      <h1>Logs</h1>
      <p>Live application &amp; Modbus TCP diagnostics (in-memory ring buffer).</p>
    </div>
    <div class="tia-actions">
      <label class="ctl">
        Level
        <select bind:value={level} onchange={refresh}>
          {#each levels as l}
            <option value={l}>{l}</option>
          {/each}
        </select>
      </label>
      <label class="ctl"><input type="checkbox" bind:checked={auto} /> Auto</label>
      <label class="ctl"><input type="checkbox" bind:checked={paused} /> Freeze</label>
      <span class="tia-badge">{plc.logs.length} lines</span>
      <button type="button" class="tia-btn" onclick={refresh}>Refresh</button>
      <button type="button" class="tia-btn" onclick={copyAll}>Copy</button>
      <button type="button" class="tia-btn" onclick={clearLogs}>Clear</button>
    </div>
  </div>

  <div class="logs-scroll" bind:this={scroller}>
    {#if plc.logs.length === 0}
      <p class="empty">
        No log lines at this level yet. Start the simulation or the Modbus slave and connect a
        client — connection, request and error events appear here.
      </p>
    {:else}
      <table class="tia-table logs-table">
        <thead>
          <tr>
            <th class="col-ts">Time</th>
            <th class="col-lvl">Level</th>
            <th class="col-src">Source</th>
            <th>Message</th>
          </tr>
        </thead>
        <tbody>
          {#each plc.logs as line (line.seq)}
            <tr>
              <td class="col-ts mono">{line.ts}</td>
              <td class="col-lvl"
                ><span class={`lvl lvl-${line.level}`}>{line.level.toUpperCase()}</span></td
              >
              <td class="col-src mono">{line.target}</td>
              <td class="msg"
                >{line.message}{#if line.fields}<span class="fields">{line.fields}</span>{/if}</td
              >
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  .logs-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--tia-paper);
  }
  .tia-actions .ctl {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--tia-muted);
  }
  .tia-actions select {
    font-size: 12px;
    padding: 2px 4px;
    border: 1px solid var(--tia-border);
    border-radius: 3px;
    background: #fff;
  }
  .logs-scroll {
    flex: 1;
    overflow: auto;
    padding: 8px;
  }
  .empty {
    padding: 20px;
    color: var(--tia-muted);
    font-size: 12px;
    max-width: 640px;
  }
  .logs-table {
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .logs-table td {
    vertical-align: top;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .col-ts {
    white-space: nowrap;
    width: 1%;
    color: var(--tia-muted);
  }
  .col-lvl {
    white-space: nowrap;
    width: 1%;
    text-align: center;
  }
  .col-src {
    white-space: nowrap;
    width: 1%;
    color: var(--tia-blue-dark);
  }
  .mono {
    font-family: var(--font-mono);
  }
  .fields {
    color: var(--tia-muted);
    margin-left: 6px;
  }
  .lvl {
    display: inline-block;
    min-width: 44px;
    padding: 0 4px;
    border-radius: 3px;
    font-weight: 700;
    font-size: 10px;
    color: #fff;
  }
  .lvl-error {
    background: var(--tia-fault);
  }
  .lvl-warn {
    background: var(--tia-warn);
  }
  .lvl-info {
    background: var(--tia-online);
  }
  .lvl-debug {
    background: var(--tia-blue-mid);
  }
  .lvl-trace {
    background: var(--tia-muted);
  }
</style>
