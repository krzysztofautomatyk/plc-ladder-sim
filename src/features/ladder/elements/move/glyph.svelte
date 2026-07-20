<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "move" }>> = $props();
</script>

<FunctionBlockBox
  title="MOVE"
  rows={[
    { k: "IN", v: formatAddress(element.source) },
    { k: "OUT", v: formatAddress(element.dest) },
  ]}
  hot={active}
/>
