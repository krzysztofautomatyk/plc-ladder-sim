<script lang="ts">
  import FunctionBlockBox from "../../components/shared/FunctionBlockBox.svelte";
  import type { ElementRenderProps } from "../_shared/types";
  import type { LadderElement } from "../../../../shared/lib/types";

  let {
    element,
    active = false,
  }: ElementRenderProps<Extract<LadderElement, { type: "tof" }>> = $props();
</script>

<FunctionBlockBox title="TOF" lines={["IN ——— Q", `PT = ${element.preset_ms} ms`]} hot={active} />
