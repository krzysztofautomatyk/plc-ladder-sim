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
  }: ElementRenderProps<Extract<LadderElement, { type: "math" }>> = $props();

  const aLabel = $derived(formatLiveOperand(plc.memory, element.a, formatAddress));
  const bLabel = $derived(formatLiveOperand(plc.memory, element.b, formatAddress));
  const dLabel = $derived(formatLiveOperand(plc.memory, element.dest, formatAddress));
</script>

<FunctionBlockBox
  title={element.op.toUpperCase()}
  rows={[
    { k: "IN1", v: aLabel, live: true },
    { k: "IN2", v: bLabel, live: true },
    { k: "OUT", v: dLabel, live: true },
  ]}
  hot={active}
/>
