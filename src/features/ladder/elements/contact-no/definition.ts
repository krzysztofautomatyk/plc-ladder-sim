import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const contactNoDefinition: ElementDefinition<"contact_no"> = {
  kind: "contact_no",
  category: "bit_logic",
  label: "NO contact",
  shortLabel: "NO",
  help: "Normally open (XIC). Conducts when the address is TRUE.",
  cellClass: "contact",
  create: () => ({
    type: "contact_no",
    id: newId("c"),
    address: { area: "discrete", index: 0 },
  }),
  topLabel: (el, fmt) => fmt(el.address),
  bottomLabel: () => "",
};
