<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../../shared/lib/api";
  import type { AuditEntry, AuditReport } from "../../shared/lib/types";

  let entries = $state<AuditEntry[]>([]);
  let chainValid = $state<boolean | null>(null);
  let loading = $state(false);
  let message = $state("");

  async function refresh() {
    loading = true;
    const [list, verify] = await Promise.all([api.getAudit(100), api.verifyAudit()]);
    loading = false;
    if (list.ok && list.data) entries = list.data.reverse();
    if (verify.ok && verify.data != null) chainValid = verify.data;
  }

  async function exportReport() {
    const res = await api.exportAudit();
    if (res.ok && res.data) {
      const report = res.data as AuditReport;
      const blob = new Blob([JSON.stringify(report, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `audit_report_${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
      message = `Exported ${report.entry_count} · chain ${report.chain_valid ? "VALID" : "INVALID"}`;
      await refresh();
    } else {
      message = res.error ?? "export failed";
    }
  }

  onMount(() => {
    refresh();
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  });
</script>

<div class="tia-page-header">
  <div>
    <h1>Audit trail</h1>
    <p>Hash-chained operator actions (regulated-style log).</p>
  </div>
  <div class="tia-actions">
    <button type="button" class="tia-btn" disabled={loading} onclick={refresh}>Refresh</button>
    <button type="button" class="tia-btn tia-btn-primary" onclick={exportReport}>Export report</button>
    {#if chainValid === true}
      <span class="tia-badge run">CHAIN VALID</span>
    {:else if chainValid === false}
      <span class="tia-badge fault">CHAIN BROKEN</span>
    {/if}
  </div>
</div>
{#if message}
  <p style="padding:0 12px;font-size:11px;color:var(--tia-muted)">{message}</p>
{/if}
<div class="tia-table-wrap">
  <table class="tia-table">
    <thead>
      <tr>
        <th>#</th>
        <th>Time</th>
        <th>Action</th>
        <th>Detail</th>
        <th>Hash</th>
      </tr>
    </thead>
    <tbody>
      {#each entries as e (e.id)}
        <tr>
          <td>{e.sequence}</td>
          <td style="font-family:var(--font-mono);font-size:11px"
            >{new Date(e.timestamp).toLocaleString()}</td
          >
          <td><strong style="color:var(--tia-blue)">{e.action}</strong></td>
          <td>{e.detail}</td>
          <td style="font-family:var(--font-mono);font-size:10px" title={e.entry_hash}
            >{e.entry_hash.slice(0, 12)}…</td
          >
        </tr>
      {/each}
    </tbody>
  </table>
</div>
