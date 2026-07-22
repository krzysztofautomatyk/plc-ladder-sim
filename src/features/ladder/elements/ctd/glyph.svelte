<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";
  import { readCounterCv, readCounterQ } from "../../lib/memoryRead";
  import { plc } from "../../../../shared/stores/plc.svelte";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "ctd" }>> = $props();

  const mem = $derived(plc.memory);
  const online = $derived(
    mem.run_state === "run" || plc.status?.run_state === "run" || plc.status?.running === true
  );
  const cv = $derived.by(() => {
    void mem.scan_count;
    return readCounterCv(mem, element.counter_index);
  });
  const cq = $derived.by(() => {
    void mem.scan_count;
    return readCounterQ(mem, element.counter_index);
  });
</script>

<FunctionBlockBox
  title="CTD"
  subtitle={`C${element.counter_index}`}
  online={online}
  rows={[
    { k: "PV", v: `${element.preset}` },
    { k: "CV", v: "", val: `${cv}`, live: true },
    { k: "Q", v: "", val: cq ? "1" : "0", live: true },
    { k: "→", v: element.done_address ? formatAddress(element.done_address) : "—" },
  ]}
  hot={active || cq}
/>
