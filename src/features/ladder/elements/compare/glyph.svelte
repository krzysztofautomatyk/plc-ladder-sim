<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";

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
</script>

<FunctionBlockBox
  title="CMP"
  lines={[CMP_LABEL[element.op] ?? element.op, formatAddress(element.a), formatAddress(element.b)]}
  hot={active}
/>
