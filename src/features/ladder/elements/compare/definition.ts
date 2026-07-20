import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

const CMP_LABEL: Record<string, string> = {
  eq: "==",
  ne: "<>",
  gt: ">",
  ge: ">=",
  lt: "<",
  le: "<=",
};

export const compareDefinition: ElementDefinition<"compare"> = {
  kind: "compare",
  category: "math",
  label: "Compare",
  shortLabel: "<>",
  help: "Compare A and B; power continues when relation holds.",
  cellClass: "fb",
  create: () => ({
    type: "compare",
    id: newId("cmp"),
    op: "ge",
    a: { area: "holding", index: 0 },
    b: { area: "holding", index: 1 },
  }),
  topLabel: (el) => `CMP ${CMP_LABEL[el.op] ?? el.op}`,
  bottomLabel: (el, fmt) => `${fmt(el.a)}  ${fmt(el.b)}`,
};
