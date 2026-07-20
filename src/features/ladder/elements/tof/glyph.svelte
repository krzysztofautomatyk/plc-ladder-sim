<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "tof" }>> = $props();
</script>

<FunctionBlockBox
  title="TOF"
  subtitle={`T${element.timer_index}`}
  rows={[
    { k: "PT", v: `${element.preset_ms}ms` },
    { k: "Q", v: element.done_address ? formatAddress(element.done_address) : "—" },
  ]}
  hot={active}
/>
