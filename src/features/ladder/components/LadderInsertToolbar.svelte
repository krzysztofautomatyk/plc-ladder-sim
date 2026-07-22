<script lang="ts">
  /**
   * Classic ladder instruction toolbar (icon strip) — insert into the selected
   * network / OR branch. Mirrors the palette but as a horizontal quick-insert bar.
   */
  import type { PaletteKind } from "../../../shared/lib/types";
  import type { ElementType } from "../elements/_shared/types";
  import { getRegistryEntry } from "../elements";
  import { shortcutFor } from "../lib/shortcuts";

  interface Props {
    onInsert: (kind: PaletteKind) => void;
    onInsertParallel: () => void;
    onAddNetwork: () => void;
    onDeleteNetwork: () => void;
    targetLabel: string;
  }
  let { onInsert, onInsertParallel, onAddNetwork, onDeleteNetwork, targetLabel }: Props =
    $props();

  const bitTools: ElementType[] = [
    "contact_no",
    "contact_nc",
    "contact_rising",
    "contact_falling",
    "coil",
    "coil_negated",
    "coil_set",
    "coil_reset",
    "wire",
  ];
  const fbTools: { kind: ElementType; label: string }[] = [
    { kind: "ton", label: "TON" },
    { kind: "tof", label: "TOF" },
    { kind: "rto", label: "RTO" },
    { kind: "ctu", label: "CTU" },
    { kind: "ctd", label: "CTD" },
    { kind: "math", label: "MATH" },
    { kind: "move", label: "MOVE" },
    { kind: "compare", label: "CMP" },
  ];

  const stroke = {
    strokeIn: "#003d5c",
    strokeOut: "#003d5c",
    strokeBody: "#1a1a1a",
    fillCoil: "none" as string | undefined,
    sw: 1.6,
  };
</script>

<div class="ins-bar">
  <span class="target" title="New instructions are inserted here">
    insert → <strong>{targetLabel}</strong>
  </span>

  <div class="grp">
    {#each bitTools as kind}
      {@const entry = getRegistryEntry(kind)}
      {@const Glyph = entry.Glyph}
      {@const preview = entry.def.create()}
      {@const key = shortcutFor(kind)}
      <button
        type="button"
        class="tool"
        title={`${entry.def.label}${key ? ` — key: ${key}` : ""}`}
        onclick={() => onInsert(kind)}
      >
        <span class="g" class:wire={kind === "wire"}>
          <Glyph
            element={preview}
            strokeIn={stroke.strokeIn}
            strokeOut={stroke.strokeOut}
            strokeBody={stroke.strokeBody}
            fillCoil={stroke.fillCoil}
            sw={stroke.sw}
            active={false}
          />
        </span>
        {#if key}<kbd>{key}</kbd>{/if}
      </button>
    {/each}
  </div>

  <button
    type="button"
    class="tool or-tool"
    title="Add a leading parallel OR branch (seal-in) — key: o"
    onclick={() => onInsert("or_branch")}
  >
    <span class="or-ico">⟇</span>
    <span class="or-lbl">OR branch</span>
    <kbd>o</kbd>
  </button>

  <button
    type="button"
    class="tool or-tool"
    title="Insert an inline parallel block anywhere in the rung"
    onclick={() => onInsertParallel()}
  >
    <span class="or-ico">▚</span>
    <span class="or-lbl">Parallel</span>
  </button>

  <div class="grp fb">
    {#each fbTools as t}
      <button
        type="button"
        class="tool txt"
        title={getRegistryEntry(t.kind).def.label}
        onclick={() => onInsert(t.kind)}>{t.label}</button
      >
    {/each}
  </div>

  <div class="spacer"></div>

  <button type="button" class="tool net" title="Insert new network" onclick={onAddNetwork}
    >＋ Network</button
  >
  <button
    type="button"
    class="tool net danger"
    title="Delete selected network"
    onclick={onDeleteNetwork}>🗑 Network</button
  >
</div>

<style>
  .ins-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    padding: 5px 8px;
    background: linear-gradient(180deg, #eef3f7, #dde5eb);
    border-bottom: 1px solid #9aa3ab;
    flex-shrink: 0;
    z-index: 6;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }
  .target {
    font-size: 11px;
    color: #5a6570;
    margin-right: 4px;
    white-space: nowrap;
  }
  .target strong {
    color: #005f87;
  }
  .grp {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 4px;
    border-left: 1px solid #c0c7ce;
  }
  .grp.fb {
    flex-wrap: wrap;
  }
  .tool {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    border: 1px solid #b3bcc4;
    background: linear-gradient(180deg, #fff, #eef1f4);
    border-radius: 3px;
    padding: 2px 5px;
    min-height: 30px;
    cursor: pointer;
    color: #1a1a1a;
  }
  .tool:hover {
    border-color: #0078a8;
    background: #e8f4fa;
  }
  .tool:active {
    background: #d3ebf6;
  }
  .g {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 24px;
    overflow: hidden;
    transform: scale(0.66);
    transform-origin: center;
  }
  .g.wire {
    width: 22px;
  }
  kbd {
    font-family: Consolas, monospace;
    font-size: 9px;
    line-height: 1;
    color: #5a6570;
    background: #f4f7f9;
    border: 1px solid #c5ccd2;
    border-bottom-width: 2px;
    border-radius: 3px;
    padding: 1px 3px;
  }
  .or-tool {
    font-size: 11px;
    font-weight: 600;
    color: #7a4b00;
  }
  .or-ico {
    font-size: 15px;
    line-height: 1;
    color: #b26a00;
  }
  .tool.txt {
    font-size: 11px;
    font-weight: 700;
    font-family: Consolas, monospace;
    color: #003d5c;
    min-width: 40px;
    justify-content: center;
  }
  .tool.net {
    font-size: 11px;
    font-weight: 600;
  }
  .tool.danger {
    color: #a00;
  }
  .tool.danger:hover {
    border-color: #a00;
    background: #fbecec;
  }
  .spacer {
    flex: 1 1 auto;
  }
</style>
