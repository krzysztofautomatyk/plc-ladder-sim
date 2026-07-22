<script lang="ts">
  /** Standard output coil ( ). */
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    strokeIn,
    strokeOut,
    strokeBody,
    fillCoil = "none",
    sw = 2,
  }: ElementRenderProps<Extract<LadderElement, { type: "coil" }>> = $props();
</script>

<svg viewBox="0 0 56 32" width="56" height="32" aria-hidden="true" overflow="visible">
  <line x1="0" y1="16" x2="12" y2="16" stroke={strokeIn} stroke-width={sw} stroke-linecap="square" />
  <circle cx="28" cy="16" r="11" fill={fillCoil} stroke={strokeBody} stroke-width={sw} />
  <line x1="44" y1="16" x2="56" y2="16" stroke={strokeOut} stroke-width={sw} stroke-linecap="square" />
</svg>
