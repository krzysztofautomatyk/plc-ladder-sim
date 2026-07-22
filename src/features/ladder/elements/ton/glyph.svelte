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

  const mem = $derived(plc.memory);
  const online = $derived(
    mem.run_state === "run" || plc.status?.run_state === "run" || plc.status?.running === true
  );
  const et = $derived.by(() => {
    void mem.scan_count;
    return readTimerEt(mem, element.timer_index);
  });
  const tq = $derived.by(() => {
    void mem.scan_count;
    return readTimerQ(mem, element.timer_index);
  });
</script>

<FunctionBlockBox
  title="TON"
  subtitle={`T${element.timer_index}`}
  online={online}
  rows={[
    { k: "PT", v: `${element.preset_ms}` },
    { k: "ET", v: "ms", val: `${et}`, live: true },
    { k: "Q", v: "", val: tq ? "1" : "0", live: true },
    { k: "→", v: element.done_address ? formatAddress(element.done_address) : "—" },
  ]}
  hot={active || tq}
/>
