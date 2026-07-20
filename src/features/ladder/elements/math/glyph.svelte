<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "math" }>> = $props();
</script>

<FunctionBlockBox
  title={element.op.toUpperCase()}
  rows={[
    { k: "IN1", v: formatAddress(element.a) },
    { k: "IN2", v: formatAddress(element.b) },
    { k: "OUT", v: formatAddress(element.dest) },
  ]}
  hot={active}
/>
