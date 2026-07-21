<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";
  import { formatLiveOperand } from "../../lib/memoryRead";
  import { plc } from "../../../../shared/stores/plc.svelte";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "move" }>> = $props();

  // Always show process-image values (updates every scan while RUN).
  const inLabel = $derived(formatLiveOperand(plc.memory, element.source, formatAddress));
  const outLabel = $derived(formatLiveOperand(plc.memory, element.dest, formatAddress));
</script>

<FunctionBlockBox
  title="MOVE"
  rows={[
    { k: "IN", v: inLabel, live: true },
    { k: "OUT", v: outLabel, live: true },
  ]}
  hot={active}
/>
