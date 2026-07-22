<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";
  import { formatAddress } from "../../lib/addressFormat";
  import { formatLiveParts } from "../../lib/memoryRead";
  import { plc } from "../../../../shared/stores/plc.svelte";

  const CMP_LABEL: Record<string, string> = {
    eq: "==",
    ne: "<>",
    gt: ">",
    ge: ">=",
    lt: "<",
    le: "<=",
  };

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "compare" }>> = $props();

  const mem = $derived(plc.memory);
  const online = $derived(
    mem.run_state === "run" || plc.status?.run_state === "run" || plc.status?.running === true
  );
  const a = $derived.by(() => {
    void mem.scan_count;
    void mem.holding_registers.length;
    return formatLiveParts(mem, element.a, formatAddress);
  });
  const b = $derived.by(() => {
    void mem.scan_count;
    void mem.holding_registers.length;
    return formatLiveParts(mem, element.b, formatAddress);
  });
</script>

<FunctionBlockBox
  title={CMP_LABEL[element.op] ?? element.op}
  online={online}
  rows={[
    { k: "IN1", v: a.addr, val: a.val, live: true },
    { k: "IN2", v: b.addr, val: b.val, live: true },
  ]}
  hot={active}
/>
