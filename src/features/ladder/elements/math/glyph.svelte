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
  lines={[`IN1 ${formatAddress(element.a)}`, `IN2 ${formatAddress(element.b)}`, `OUT ${formatAddress(element.dest)}`]}
  hot={active}
/>
