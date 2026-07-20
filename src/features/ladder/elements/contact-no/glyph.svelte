<script lang="ts">
  /** IEC normally-open contact glyph. */
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    strokeIn,
    strokeOut,
    strokeBody,
    sw = 2,
  }: ElementRenderProps<Extract<LadderElement, { type: "contact_no" }>> = $props();
</script>

<svg viewBox="0 0 56 32" width="56" height="32" aria-hidden="true">
  <line x1="0" y1="16" x2="16" y2="16" stroke={strokeIn} stroke-width={sw} />
  <line x1="16" y1="4" x2="16" y2="28" stroke={strokeBody} stroke-width={sw} stroke-linecap="butt" />
  <line x1="40" y1="4" x2="40" y2="28" stroke={strokeBody} stroke-width={sw} stroke-linecap="butt" />
  <line x1="40" y1="16" x2="56" y2="16" stroke={strokeOut} stroke-width={sw} />
</svg>
