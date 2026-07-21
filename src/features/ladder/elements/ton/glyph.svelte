<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";
  import { readTimerEt, readTimerQ } from "../../lib/memoryRead";
  import { plc } from "../../../../shared/stores/plc.svelte";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "ton" }>> = $props();

  const et = $derived(readTimerEt(plc.memory, element.timer_index));
  const tq = $derived(readTimerQ(plc.memory, element.timer_index));
</script>

<FunctionBlockBox
  title="TON"
  subtitle={`T${element.timer_index}`}
  rows={[
    { k: "PT", v: `${element.preset_ms}` },
    { k: "ET", v: `${et}`, live: true },
    { k: "Q", v: tq ? "1" : "0", live: true },
    { k: "→", v: element.done_address ? formatAddress(element.done_address) : "—" },
  ]}
  hot={active || tq}
/>
