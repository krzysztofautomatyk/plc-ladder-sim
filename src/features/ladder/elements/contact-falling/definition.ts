import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const contactFallingDefinition: ElementDefinition<"contact_falling"> = {
  kind: "contact_falling",
  category: "bit_logic",
  label: "Falling edge ↓",
  shortLabel: "N",
  help: "Negative transition (N). TRUE for one scan on 1→0.",
  cellClass: "edge",
  create: () => ({
    type: "contact_falling",
    id: newId("n"),
    address: { area: "discrete", index: 0 },
  }),
  topLabel: (el, fmt) => fmt(el.address),
  bottomLabel: () => "N  ↓  1→0",
};
