import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const rtoDefinition: ElementDefinition<"rto"> = {
  kind: "rto",
  category: "timer",
  label: "RTO",
  shortLabel: "RTO",
  help: "Retentive on-delay. ET holds when IN is FALSE; reset clears.",
  cellClass: "fb",
  create: () => ({
    type: "rto",
    id: newId("rto"),
    preset_ms: 2000,
    timer_index: 2,
    done_address: { area: "coil", index: 4 },
    reset_address: { area: "discrete", index: 4 },
  }),
  topLabel: (el) => `T${el.timer_index}`,
  bottomLabel: (el) => `PT ${el.preset_ms} ms`,
};
