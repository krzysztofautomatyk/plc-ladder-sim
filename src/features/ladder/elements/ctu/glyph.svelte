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
  }: ElementRenderProps<Extract<LadderElement, { type: "ctu" }>> = $props();

  const cv = $derived(readCounterCv(plc.memory, element.counter_index));
  const cq = $derived(readCounterQ(plc.memory, element.counter_index));
</script>

<FunctionBlockBox
  title="CTU"
  subtitle={`C${element.counter_index}`}
  rows={[
    { k: "PV", v: `${element.preset}` },
    { k: "CV", v: `${cv}`, live: true },
    { k: "Q", v: cq ? "1" : "0", live: true },
    { k: "→", v: element.done_address ? formatAddress(element.done_address) : "—" },
  ]}
  hot={active || cq}
/>
