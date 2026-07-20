<script lang="ts">
  /**
   * Generic TIA-style function block box used by timers, counters, math, move, compare.
   */
  interface Props {
    title: string;
    lines: string[];
    hot?: boolean;
  }
  let { title, lines, hot = false }: Props = $props();
</script>

<div class="fb" class:hot>
  <div class="fb-h">{title}</div>
  <div class="fb-b">
    {#each lines as line}
      <div class="dim">{line}</div>
    {/each}
  </div>
</div>

<style>
  .fb {
    border: 2px solid #1a1a1a;
    background: #fff;
    min-width: 90px;
  }
  .fb.hot {
    border-color: #00a651;
    background: #f3fbf6;
  }
  .fb-h {
    font-size: 11px;
    font-weight: 800;
    text-align: center;
    padding: 2px 5px;
    border-bottom: 1px solid #1a1a1a;
    background: linear-gradient(180deg, #eef3f7, #dde5eb);
    color: #003d5c;
    letter-spacing: 0.04em;
  }
  .fb.hot .fb-h {
    border-bottom-color: #00a651;
    background: linear-gradient(180deg, #e0f5ea, #c8ecd8);
    color: #006633;
  }
  .fb-b {
    font-family: Consolas, monospace;
    font-size: 9px;
    padding: 3px 5px 4px;
    line-height: 1.35;
    color: #222;
  }
  .dim {
    color: #5a6570;
  }
</style>
