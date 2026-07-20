<script lang="ts">
  /** SET / latch coil (S). */
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    strokeIn,
    strokeOut,
    strokeBody,
    fillCoil = "none",
    sw = 2,
  }: ElementRenderProps<Extract<LadderElement, { type: "coil_set" }>> = $props();
</script>

<svg viewBox="0 0 56 32" width="56" height="32" aria-hidden="true">
  <line x1="0" y1="16" x2="12" y2="16" stroke={strokeIn} stroke-width={sw} />
  <circle cx="28" cy="16" r="11" fill={fillCoil} stroke={strokeBody} stroke-width={sw} />
  <path
    d="M32.5 11.5 C31 10.2 29 10 27.5 10.8 C25.5 11.8 25.2 13.5 26.5 14.5 C27.5 15.3 29.5 15.6 31 16.5 C32.8 17.6 33.2 19.8 31.5 21.2 C30 22.5 27.5 22.5 25.8 21.2"
    fill="none"
    stroke={strokeBody}
    stroke-width="2"
    stroke-linecap="round"
  />
  <line x1="44" y1="16" x2="56" y2="16" stroke={strokeOut} stroke-width={sw} />
</svg>
