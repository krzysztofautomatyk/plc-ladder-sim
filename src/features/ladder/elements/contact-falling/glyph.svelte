<script lang="ts">
  /** Falling edge (N) contact — step 1→0. */
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    strokeIn,
    strokeOut,
    strokeBody,
    sw = 2,
  }: ElementRenderProps<Extract<LadderElement, { type: "contact_falling" }>> = $props();
</script>

<svg viewBox="0 0 64 36" width="64" height="36" aria-hidden="true" overflow="visible">
  <line x1="0" y1="18" x2="13" y2="18" stroke={strokeIn} stroke-width={sw} stroke-linecap="square" />
  <line x1="51" y1="18" x2="64" y2="18" stroke={strokeOut} stroke-width={sw} stroke-linecap="square" />
  <line x1="13" y1="5" x2="13" y2="31" stroke={strokeBody} stroke-width="2.4" stroke-linecap="butt" />
  <line x1="51" y1="5" x2="51" y2="31" stroke={strokeBody} stroke-width="2.4" stroke-linecap="butt" />
  <path
    d="M18 9 H28 V27 H46"
    fill="none"
    stroke={strokeBody}
    stroke-width="2.35"
    stroke-linejoin="miter"
    stroke-linecap="square"
  />
  <path
    d="M25 22 L28 28 L31 22"
    fill="none"
    stroke={strokeBody}
    stroke-width="1.8"
    stroke-linejoin="miter"
  />
</svg>
