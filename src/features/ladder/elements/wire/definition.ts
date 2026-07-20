import type { ElementDefinition } from "../_shared/types";
import { newId } from "../_shared/id";

export const wireDefinition: ElementDefinition<"wire"> = {
  kind: "wire",
  category: "wire",
  label: "Open branch",
  shortLabel: "──",
  help: "Horizontal continuity wire / open branch placeholder.",
  cellClass: "wire",
  create: () => ({
    type: "wire",
    id: newId("w"),
  }),
  topLabel: () => "",
  bottomLabel: () => "",
};
