<script lang="ts">
  /** RESET / unlatch coil (R). */
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

<svg viewBox="0 0 56 32" width="56" height="32" aria-hidden="true">
  <line x1="0" y1="16" x2="12" y2="16" stroke={strokeIn} stroke-width={sw} />
  <circle cx="28" cy="16" r="11" fill={fillCoil} stroke={strokeBody} stroke-width={sw} />
  <path
    d="M24 22 V10 H29.5 C32.2 10 33.5 11.6 33.5 13.5 C33.5 15.2 32.3 16.6 30 16.8 L34 22"
    fill="none"
    stroke={strokeBody}
    stroke-width="2"
    stroke-linejoin="round"
    stroke-linecap="round"
  />
  <line x1="24" y1="10" x2="24" y2="22" stroke={strokeBody} stroke-width="2" />
  <line x1="44" y1="16" x2="56" y2="16" stroke={strokeOut} stroke-width={sw} />
</svg>
