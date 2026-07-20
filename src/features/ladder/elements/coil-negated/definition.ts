import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const coilNegatedDefinition: ElementDefinition<"coil_negated"> = {
  kind: "coil_negated",
  category: "bit_logic",
  label: "Negated coil",
  shortLabel: "(/)",
  help: "Negated coil. Writes the inverted rung power state.",
  cellClass: "coil",
  create: () => ({
    type: "coil_negated",
    id: newId("q"),
    address: { area: "coil", index: 1 },
  }),
  topLabel: (el, fmt) => fmt(el.address),
  bottomLabel: () => "",
};
