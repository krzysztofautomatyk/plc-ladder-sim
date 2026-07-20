import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const coilResetDefinition: ElementDefinition<"coil_reset"> = {
  kind: "coil_reset",
  category: "bit_logic",
  label: "RESET (R)",
  shortLabel: "(R)",
  help: "RESET / unlatch. When power is TRUE, the bit clears to 0.",
  cellClass: "coil",
  create: () => ({
    type: "coil_reset",
    id: newId("r"),
    address: { area: "coil", index: 0 },
  }),
  topLabel: (el, fmt) => fmt(el.address),
  bottomLabel: () => "RESET  R",
};
