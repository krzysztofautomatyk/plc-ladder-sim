import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const ctdDefinition: ElementDefinition<"ctd"> = {
  kind: "ctd",
  category: "counter",
  label: "CTD",
  shortLabel: "CTD",
  help: "Count-down. CV starts at preset; decrements on rising edge of CD.",
  cellClass: "fb",
  create: () => ({
    type: "ctd",
    id: newId("ctd"),
    preset: 10,
    counter_index: 12,
    done_address: { area: "coil", index: 5 },
    load_address: { area: "discrete", index: 5 },
  }),
  topLabel: (el) => `C${el.counter_index}`,
  bottomLabel: (el) => `PV ${el.preset}`,
};
