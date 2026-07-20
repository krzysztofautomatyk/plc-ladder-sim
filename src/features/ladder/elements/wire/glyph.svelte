<script lang="ts">
  /** Horizontal continuity wire. */
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    strokeBody,
    sw = 2,
  }: ElementRenderProps<Extract<LadderElement, { type: "wire" }>> = $props();
</script>

<svg viewBox="0 0 28 32" width="28" height="32" aria-hidden="true">
  <line x1="0" y1="16" x2="28" y2="16" stroke={strokeBody} stroke-width={sw} />
</svg>
