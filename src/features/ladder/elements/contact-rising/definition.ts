import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const contactRisingDefinition: ElementDefinition<"contact_rising"> = {
  kind: "contact_rising",
  category: "bit_logic",
  label: "Rising edge ↑",
  shortLabel: "P",
  help: "Positive transition (P). TRUE for one scan on 0→1.",
  cellClass: "edge",
  create: () => ({
    type: "contact_rising",
    id: newId("p"),
    address: { area: "discrete", index: 0 },
  }),
  topLabel: (el, fmt) => fmt(el.address),
  bottomLabel: () => "P  ↑  0→1",
};
