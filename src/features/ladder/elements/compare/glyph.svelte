<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";
  import { formatLiveOperand } from "../../lib/memoryRead";
  import { plc } from "../../../../shared/stores/plc.svelte";

  const CMP_LABEL: Record<string, string> = {
    eq: "==",
    ne: "<>",
    gt: ">",
    ge: ">=",
    lt: "<",
    le: "<=",
  };

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "compare" }>> = $props();

  const aLabel = $derived(formatLiveOperand(plc.memory, element.a, formatAddress));
  const bLabel = $derived(formatLiveOperand(plc.memory, element.b, formatAddress));
</script>

<FunctionBlockBox
  title={CMP_LABEL[element.op] ?? element.op}
  rows={[
    { k: "IN1", v: aLabel, live: true },
    { k: "IN2", v: bLabel, live: true },
  ]}
  hot={active}
/>
