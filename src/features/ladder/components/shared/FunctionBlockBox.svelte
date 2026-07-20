<script lang="ts">
  /**
   * Function-block glyph for timers, counters, math, move and compare —
   * a compact instruction box with a mnemonic header, parameter rows
   * (label · value) and left/right power stubs so it sits on the rung.
   */
  interface Row {
    k: string;
    v: string;
  }
  interface Props {
    title: string;
    subtitle?: string;
    rows: Row[];
    hot?: boolean;
  }
  let { title, subtitle = "", rows, hot = false }: Props = $props();
</script>

<div class="fb-wrap" class:hot>
  <i class="stub left"></i>
  <div class="fb">
    <div class="fb-h">
      <span class="mn">{title}</span>
      {#if subtitle}<span class="sub">{subtitle}</span>{/if}
    </div>
    <div class="fb-b">
      {#each rows as r}
        <div class="fb-row">
          <span class="k">{r.k}</span>
          <span class="v">{r.v}</span>
        </div>
      {/each}
    </div>
  </div>
  <i class="stub right"></i>
</div>

<style>
  .fb-wrap {
    display: flex;
    align-items: center;
  }
  .stub {
    width: 6px;
    height: 2px;
    background: #1a1a1a;
    flex-shrink: 0;
  }
  .fb-wrap.hot .stub {
    background: #00a651;
  }
  .fb {
    min-width: 96px;
    border: 1.5px solid #1a1a1a;
    background: #fff;
    border-radius: 2px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
    overflow: hidden;
  }
  .fb-wrap.hot .fb {
    border-color: #00a651;
    box-shadow: 0 0 6px rgba(0, 166, 81, 0.35);
  }
  .fb-h {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 6px;
    padding: 2px 6px;
    border-bottom: 1.5px solid #1a1a1a;
    background: linear-gradient(180deg, #2a6f9e, #1a557d);
    color: #fff;
  }
  .fb-h .mn {
    font-family: "Segoe UI", sans-serif;
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.06em;
  }
  .fb-h .sub {
    font-family: Consolas, monospace;
    font-size: 9px;
    font-weight: 700;
    color: #bfe0f2;
  }
  .fb-wrap.hot .fb-h {
    background: linear-gradient(180deg, #17a25a, #0c8347);
    border-bottom-color: #00a651;
  }
  .fb-b {
    padding: 2px 0;
  }
  .fb-row {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-family: Consolas, monospace;
    font-size: 9.5px;
    line-height: 1.55;
    padding: 0 8px;
  }
  .fb-row .k {
    color: #5a6570;
    font-weight: 600;
  }
  .fb-row .v {
    color: #003d5c;
    font-weight: 700;
  }
  .fb-wrap.hot .fb-row .v {
    color: #006633;
  }
</style>

