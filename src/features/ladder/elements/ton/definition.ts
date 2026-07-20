import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const tonDefinition: ElementDefinition<"ton"> = {
  kind: "ton",
  category: "timer",
  label: "TON",
  shortLabel: "TON",
  help: "On-delay timer. Q becomes TRUE after IN is TRUE for PT.",
  cellClass: "fb",
  create: () => ({
    type: "ton",
    id: newId("ton"),
    preset_ms: 1000,
    timer_index: 0,
    done_address: { area: "coil", index: 1 },
  }),
  topLabel: (el) => `T${el.timer_index}`,
  bottomLabel: (el) => `PT ${el.preset_ms} ms`,
};
