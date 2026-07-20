<script lang="ts">
  import type { PaletteKind } from "../../../shared/lib/types";
  import { getRegistryEntry, paletteGroups } from "../elements";

  interface Props {
    onAdd: (kind: PaletteKind) => void;
  }
  let { onAdd }: Props = $props();

  const groups = paletteGroups();

  const paletteStrokes = {
    strokeIn: "#005f87",
    strokeOut: "#005f87",
    strokeBody: "#1a1a1a",
    fillCoil: "none" as string | undefined,
    sw: 1.6,
  };

  function onDragStart(e: DragEvent, kind: PaletteKind) {
    e.dataTransfer?.setData("application/x-lad-kind", kind);
    e.dataTransfer!.effectAllowed = "copy";
  }
</script>

<div class="instr">
  {#each groups as g}
    <div class="g-title">{g.title}</div>
    {#each g.items as item}
      {@const entry = getRegistryEntry(item.type)}
      {@const Glyph = entry.Glyph}
      {@const preview = entry.def.create()}
      <button
        type="button"
        class="instr-item"
        draggable="true"
        ondragstart={(e) => onDragStart(e, item.kind)}
        onclick={() => onAdd(item.kind)}
        title={entry.def.help}
      >
        <span class="glyph" class:fb={entry.def.cellClass === "fb"}>
          {#if item.kind === "or_branch"}
            <span class="ascii">∥</span>
          {:else}
            <Glyph
              element={preview}
              strokeIn={paletteStrokes.strokeIn}
              strokeOut={paletteStrokes.strokeOut}
              strokeBody={paletteStrokes.strokeBody}
              fillCoil={paletteStrokes.fillCoil}
              sw={paletteStrokes.sw}
              active={false}
            />
          {/if}
        </span>
        <span class="lbl">{item.label}</span>
      </button>
    {/each}
  {/each}
</div>

<style>
  .instr {
    padding: 4px 6px 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .g-title {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #5a6570;
    padding: 8px 4px 3px;
  }
  .instr-item {
    display: grid;
    grid-template-columns: 44px 1fr;
    gap: 6px;
    align-items: center;
    text-align: left;
    border: 1px solid #c5ccd2;
    background: #fff;
    padding: 4px 6px;
    border-radius: 2px;
    cursor: grab;
    font-size: 11px;
    color: #1a1a1a;
  }
  .instr-item:hover {
    border-color: #0078a8;
    background: #e8f4fa;
  }
  .instr-item:active {
    cursor: grabbing;
  }
  .glyph {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 28px;
    overflow: hidden;
    transform: scale(0.72);
    transform-origin: center;
  }
  .glyph.fb {
    transform: scale(0.48);
  }
  .ascii {
    font-family: Consolas, monospace;
    font-weight: 700;
    font-size: 16px;
    color: #005f87;
  }
  .lbl {
    font-weight: 500;
  }
</style>
