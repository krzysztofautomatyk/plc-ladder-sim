import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const ctuDefinition: ElementDefinition<"ctu"> = {
  kind: "ctu",
  category: "counter",
  label: "CTU",
  shortLabel: "CTU",
  help: "Count-up. Increments on rising edge of CU until PV.",
  cellClass: "fb",
  create: () => ({
    type: "ctu",
    id: newId("ctu"),
    preset: 10,
    counter_index: 10,
    done_address: { area: "coil", index: 3 },
    reset_address: { area: "discrete", index: 3 },
  }),
  topLabel: (el) => `C${el.counter_index}`,
  bottomLabel: (el) => `PV ${el.preset}`,
};
