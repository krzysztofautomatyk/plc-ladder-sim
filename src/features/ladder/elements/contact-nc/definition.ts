import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const contactNcDefinition: ElementDefinition<"contact_nc"> = {
  kind: "contact_nc",
  category: "bit_logic",
  label: "NC contact",
  shortLabel: "NC",
  help: "Normally closed (XIO). Conducts when the address is FALSE.",
  cellClass: "contact",
  create: () => ({
    type: "contact_nc",
    id: newId("c"),
    address: { area: "discrete", index: 1 },
  }),
  topLabel: (el, fmt) => fmt(el.address),
  bottomLabel: () => "",
};
