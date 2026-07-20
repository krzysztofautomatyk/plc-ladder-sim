import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const coilDefinition: ElementDefinition<"coil"> = {
  kind: "coil",
  category: "bit_logic",
  label: "Coil",
  shortLabel: "( )",
  help: "Standard output coil. Writes the rung power state to the address.",
  cellClass: "coil",
  create: () => ({
    type: "coil",
    id: newId("q"),
    address: { area: "coil", index: 0 },
  }),
  topLabel: (el, fmt) => fmt(el.address),
  bottomLabel: () => "",
};
