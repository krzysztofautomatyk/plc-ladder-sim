import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const mathDefinition: ElementDefinition<"math"> = {
  kind: "math",
  category: "math",
  label: "ADD…DIV",
  shortLabel: "±",
  help: "Math block: ADD / SUB / MUL / DIV on word addresses.",
  cellClass: "fb",
  create: () => ({
    type: "math",
    id: newId("math"),
    op: "add",
    a: { area: "holding", index: 0 },
    b: { area: "holding", index: 1 },
    dest: { area: "holding", index: 2 },
  }),
  topLabel: (el) => el.op.toUpperCase(),
  bottomLabel: (el, fmt) => `${fmt(el.a)} → ${fmt(el.dest)}`,
};
