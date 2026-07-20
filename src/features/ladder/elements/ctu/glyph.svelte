<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "ctu" }>> = $props();
</script>

<FunctionBlockBox
  title="CTU"
  subtitle={`C${element.counter_index}`}
  rows={[
    { k: "PV", v: `${element.preset}` },
    { k: "Q", v: element.done_address ? formatAddress(element.done_address) : "—" },
  ]}
  hot={active}
/>
