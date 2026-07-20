import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const moveDefinition: ElementDefinition<"move"> = {
  kind: "move",
  category: "math",
  label: "MOVE",
  shortLabel: "→",
  help: "Copy source word to destination when power is TRUE.",
  cellClass: "fb",
  create: () => ({
    type: "move",
    id: newId("mov"),
    source: { area: "holding", index: 0 },
    dest: { area: "holding", index: 1 },
  }),
  topLabel: () => "MOVE",
  bottomLabel: (el, fmt) => `${fmt(el.source)} → ${fmt(el.dest)}`,
};
