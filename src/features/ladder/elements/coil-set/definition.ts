import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const coilSetDefinition: ElementDefinition<"coil_set"> = {
  kind: "coil_set",
  category: "bit_logic",
  label: "SET (S)",
  shortLabel: "(S)",
  help: "SET / latch. When power is TRUE, the bit stays 1 until RESET.",
  cellClass: "coil",
  create: () => ({
    type: "coil_set",
    id: newId("s"),
    address: { area: "coil", index: 0 },
  }),
  topLabel: (el, fmt) => fmt(el.address),
  bottomLabel: () => "SET  S",
};
