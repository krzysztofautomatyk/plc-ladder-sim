<script lang="ts">
  /**
   * Diagnostics — live application log viewer.
   *
   * Reads the in-memory tracing ring buffer via `get_logs`, with a level filter,
   * auto-refresh, clear and copy-to-clipboard. This is where Modbus TCP activity
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
  <div class="logs-toolbar">
    <strong>Application logs</strong>
    <label>
      Level
      <select bind:value={level} onchange={refresh}>
        {#each levels as l}
          <option value={l}>{l}</option>
        {/each}
      </select>
    </label>
    <label class="chk">
      <input type="checkbox" bind:checked={auto} /> Auto-refresh
    </label>
    <label class="chk">
      <input type="checkbox" bind:checked={paused} /> Freeze scroll
    </label>
    <span class="spacer"></span>
    <span class="count">{plc.logs.length} lines</span>
    <button type="button" onclick={refresh}>Refresh</button>
    <button type="button" onclick={copyAll}>Copy</button>
    <button type="button" class="danger" onclick={clearLogs}>Clear</button>
  </div>

  <div class="logs-body" bind:this={scroller}>
    {#if plc.logs.length === 0}
      <div class="empty">
        No log lines at this level yet. Start the simulation or the Modbus slave and connect a
        client — connection, request and error events appear here.
      </div>
    {:else}
      <table>
        <tbody>
          {#each plc.logs as line (line.seq)}
            <tr class={`lvl-${line.level}`}>
              <td class="ts">{line.ts}</td>
              <td class="lvl">{line.level.toUpperCase()}</td>
              <td class="target">{line.target}</td>
              <td class="msg">
                {line.message}
                {#if line.fields}<span class="fields">{line.fields}</span>{/if}
              </td>
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
    background: #0f1115;
    color: #d6dae0;
    font-family: "Cascadia Code", Consolas, monospace;
  }
  .logs-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 10px;
    background: #1a1d23;
    border-bottom: 1px solid #2a2e36;
    font-size: 12px;
    color: #aeb4bd;
  }
  .logs-toolbar .spacer {
    flex: 1;
  }
  .logs-toolbar label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .logs-toolbar .chk {
    user-select: none;
  }
  .logs-toolbar button {
    background: #262b33;
    color: #d6dae0;
    border: 1px solid #3a404a;
    border-radius: 4px;
    padding: 3px 10px;
    cursor: pointer;
  }
  .logs-toolbar button:hover {
    background: #303743;
  }
  .logs-toolbar button.danger {
    border-color: #6b2b2b;
    color: #ffb4b4;
  }
  .count {
    color: #7d8590;
  }
  .logs-body {
    flex: 1;
    overflow: auto;
    padding: 4px 0;
  }
  .empty {
    padding: 24px;
    color: #7d8590;
    max-width: 640px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  td {
    padding: 1px 8px;
    vertical-align: top;
    white-space: pre-wrap;
    word-break: break-word;
  }
  td.ts {
    color: #6f7680;
    white-space: nowrap;
  }
  td.lvl {
    white-space: nowrap;
    font-weight: 700;
    text-align: right;
  }
  td.target {
    color: #8a94a6;
    white-space: nowrap;
  }
  .fields {
    color: #8a94a6;
    margin-left: 6px;
  }
  tr.lvl-error td.lvl,
  tr.lvl-error td.msg {
    color: #ff6b6b;
  }
  tr.lvl-warn td.lvl,
  tr.lvl-warn td.msg {
    color: #ffcc66;
  }
  tr.lvl-info td.lvl {
    color: #66d9a6;
  }
  tr.lvl-debug td.lvl {
    color: #6fb3ff;
  }
  tr.lvl-trace td.lvl {
    color: #9aa2ad;
  }
  tr:hover {
    background: #171a20;
  }
</style>
