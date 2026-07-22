<script lang="ts">
  /** IEC normally-open contact glyph — filled square when power flows (closed). */
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    strokeIn,
    strokeOut,
    strokeBody,
    sw = 2,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "contact_no" }>> = $props();
</script>

<svg viewBox="0 0 56 32" width="56" height="32" aria-hidden="true" overflow="visible">
  <!-- Left / right power stubs -->
  <line x1="0" y1="16" x2="16" y2="16" stroke={strokeIn} stroke-width={sw} stroke-linecap="square" />
  <!-- Vertical poles -->
  <line x1="16" y1="4" x2="16" y2="28" stroke={strokeBody} stroke-width={sw} stroke-linecap="butt" />
  <line x1="40" y1="4" x2="40" y2="28" stroke={strokeBody} stroke-width={sw} stroke-linecap="butt" />
  <line x1="40" y1="16" x2="56" y2="16" stroke={strokeOut} stroke-width={sw} stroke-linecap="square" />

  {#if active}
    <!-- Conducting: solid bridge between poles (TIA online “closed contact”) -->
    <rect
      x="18"
      y="10"
      width="20"
      height="12"
      fill={strokeBody}
      stroke={strokeBody}
      stroke-width="1"
      rx="1"
    />
  {/if}
</svg>
