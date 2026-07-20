<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "rto" }>> = $props();
</script>

<FunctionBlockBox
  title="RTO"
  subtitle={`T${element.timer_index}`}
  rows={[
    { k: "PT", v: `${element.preset_ms}ms` },
    { k: "Q", v: element.done_address ? formatAddress(element.done_address) : "—" },
    { k: "R", v: element.reset_address ? formatAddress(element.reset_address) : "—" },
  ]}
  hot={active}
/>
