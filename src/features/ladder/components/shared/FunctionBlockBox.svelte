<script lang="ts">
  /**
   * Function-block glyph for timers, counters, math, move and compare —
   * compact instruction box with parameter rows and live process values.
   */
  interface Row {
    k: string;
    /** Address label (e.g. R100) or full "R100=350" if value omitted. */
    v: string;
    /** Live numeric/string value shown large on the right (TIA online). */
    val?: string;
    /** Highlight live process-image values during RUN. */
    live?: boolean;
  }
  interface Props {
    title: string;
    subtitle?: string;
    rows: Row[];
    hot?: boolean;
    /** When true, show online styling even if not power-flow hot. */
    online?: boolean;
  }
  let { title, subtitle = "", rows, hot = false, online = false }: Props = $props();
</script>

<div class="fb-wrap" class:hot class:online>
  <i class="stub left"></i>
  <div class="fb">
    <div class="fb-h">
      <span class="mn">{title}</span>
      {#if subtitle}<span class="sub">{subtitle}</span>{/if}
      {#if online}<span class="live-dot" title="Live process values">●</span>{/if}
    </div>
    <div class="fb-b">
      {#each rows as r (r.k + r.v + (r.val ?? ""))}
        <div class="fb-row" class:live={r.live}>
          <span class="k">{r.k}</span>
          <span class="mid">
            <span class="addr">{r.v}</span>
            {#if r.val != null && r.val !== ""}
              <span class="val" title="Live value">{r.val}</span>
            {/if}
          </span>
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
    width: 10px;
    height: 2px;
    background: #1a1a1a;
    flex-shrink: 0;
  }
  .fb-wrap.hot .stub,
  .fb-wrap.online .stub {
    background: #00a651;
    box-shadow: 0 0 3px rgba(0, 166, 81, 0.4);
  }
  .fb {
    min-width: 132px;
    border: 1.5px solid #1a1a1a;
    background: #fff;
    border-radius: 2px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
    overflow: hidden;
  }
  .fb-wrap.hot .fb,
  .fb-wrap.online .fb {
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
  .live-dot {
    font-size: 8px;
    color: #7dffb0;
    margin-left: 2px;
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }
  .fb-wrap.hot .fb-h,
  .fb-wrap.online .fb-h {
    background: linear-gradient(180deg, #17a25a, #0c8347);
    border-bottom-color: #00a651;
  }
  .fb-b {
    padding: 3px 0;
  }
  .fb-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    font-family: Consolas, monospace;
    font-size: 9.5px;
    line-height: 1.45;
    padding: 1px 7px;
  }
  .fb-row .k {
    color: #5a6570;
    font-weight: 600;
    flex-shrink: 0;
  }
  .mid {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    justify-content: flex-end;
  }
  .addr {
    color: #003d5c;
    font-weight: 700;
    white-space: nowrap;
  }
  .val {
    font-weight: 800;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: #0b5cab;
    background: #e3f2fd;
    border: 1px solid #90caf9;
    padding: 0 5px;
    border-radius: 3px;
    min-width: 2.4em;
    text-align: right;
    line-height: 1.5;
  }
  .fb-row.live .addr {
    color: #0b5cab;
  }
  .fb-wrap.hot .addr,
  .fb-wrap.online .addr {
    color: #006633;
  }
  .fb-wrap.hot .val,
  .fb-wrap.online .val {
    color: #0a5c30;
    background: #e8f8ef;
    border-color: #81c995;
  }
</style>
