<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "ctu" }>> = $props();
</script>

<FunctionBlockBox title="CTU" lines={["CU ——— Q", `PV = ${element.preset}`]} hot={active} />
