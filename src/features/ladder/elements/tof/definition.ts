import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const tofDefinition: ElementDefinition<"tof"> = {
  kind: "tof",
  category: "timer",
  label: "TOF",
  shortLabel: "TOF",
  help: "Off-delay timer. Q stays TRUE for PT after IN goes FALSE.",
  cellClass: "fb",
  create: () => ({
    type: "tof",
    id: newId("tof"),
    preset_ms: 1000,
    timer_index: 1,
    done_address: { area: "coil", index: 2 },
  }),
  topLabel: (el) => `T${el.timer_index}`,
  bottomLabel: (el) => `PT ${el.preset_ms} ms`,
};
