<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";
  import { formatLiveParts } from "../../lib/memoryRead";
  import { plc } from "../../../../shared/stores/plc.svelte";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "move" }>> = $props();

  // Depend on scan_count so values refresh every PLC scan while RUN.
  const mem = $derived(plc.memory);
  const online = $derived(
    mem.run_state === "run" || plc.status?.run_state === "run" || plc.status?.running === true
  );
  const src = $derived.by(() => {
    void mem.scan_count;
    void mem.holding_registers.length;
    return formatLiveParts(mem, element.source, formatAddress);
  });
  const dst = $derived.by(() => {
    void mem.scan_count;
    void mem.holding_registers.length;
    return formatLiveParts(mem, element.dest, formatAddress);
  });
</script>

<FunctionBlockBox
  title="MOVE"
  online={online}
  rows={[
    { k: "IN", v: src.addr, val: src.val, live: true },
    { k: "OUT", v: dst.addr, val: dst.val, live: true },
  ]}
  hot={active}
/>
