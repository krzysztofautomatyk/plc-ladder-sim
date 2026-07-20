<script lang="ts">
  /** Rising edge (P) contact — step 0→1. */
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    strokeIn,
    strokeOut,
    strokeBody,
    sw = 2,
  }: ElementRenderProps<Extract<LadderElement, { type: "contact_rising" }>> = $props();
</script>

<svg viewBox="0 0 64 36" width="64" height="36" aria-hidden="true">
  <line x1="0" y1="18" x2="13" y2="18" stroke={strokeIn} stroke-width={sw} />
  <line x1="51" y1="18" x2="64" y2="18" stroke={strokeOut} stroke-width={sw} />
  <line x1="13" y1="5" x2="13" y2="31" stroke={strokeBody} stroke-width="2.4" stroke-linecap="butt" />
  <line x1="51" y1="5" x2="51" y2="31" stroke={strokeBody} stroke-width="2.4" stroke-linecap="butt" />
  <path
    d="M18 27 H28 V9 H46"
    fill="none"
    stroke={strokeBody}
    stroke-width="2.35"
    stroke-linejoin="miter"
    stroke-linecap="square"
  />
  <path
    d="M25 14 L28 8 L31 14"
    fill="none"
    stroke={strokeBody}
    stroke-width="1.8"
    stroke-linejoin="miter"
  />
</svg>
