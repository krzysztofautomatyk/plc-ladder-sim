<script lang="ts">
  /** RESET / unlatch coil (R) — TIA-style circle with crisp capital R. */
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    strokeIn,
    strokeOut,
    strokeBody,
    fillCoil = "none",
    sw = 2,
  }: ElementRenderProps<Extract<LadderElement, { type: "coil_reset" }>> = $props();
</script>

<svg viewBox="0 0 56 32" width="56" height="32" aria-hidden="true" overflow="visible">
  <line x1="0" y1="16" x2="12" y2="16" stroke={strokeIn} stroke-width={sw} stroke-linecap="square" />
  <circle cx="28" cy="16" r="11" fill={fillCoil} stroke={strokeBody} stroke-width={sw} />
  <!-- Typographic R — matches SET(S) weight and centering -->
  <text
    x="28"
    y="16.5"
    text-anchor="middle"
    dominant-baseline="central"
    fill={strokeBody}
    font-family="Segoe UI, system-ui, Arial, sans-serif"
    font-size="13"
    font-weight="800"
    letter-spacing="0"
    style="user-select: none"
  >R</text>
  <line x1="44" y1="16" x2="56" y2="16" stroke={strokeOut} stroke-width={sw} stroke-linecap="square" />
</svg>
