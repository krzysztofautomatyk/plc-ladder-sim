/**
 * Central instruction registry — one entry per IEC ladder element.
 * New instruction = new folder under elements/ + one line here.
 */
import type { LadderElement, PaletteKind } from "../../../../shared/lib/types";
import type {
  ElementGlyph,
  ElementDefinition,
  ElementType,
  RegistryEntry,
} from "./types";

import * as contactNo from "../contact-no";
import * as contactNc from "../contact-nc";
import * as contactRising from "../contact-rising";
import * as contactFalling from "../contact-falling";
import * as coil from "../coil";
import * as coilNegated from "../coil-negated";
import * as coilSet from "../coil-set";
import * as coilReset from "../coil-reset";
import * as wire from "../wire";
import * as ton from "../ton";
import * as tof from "../tof";
import * as rto from "../rto";
import * as ctu from "../ctu";
import * as ctd from "../ctd";
import * as math from "../math";
import * as move from "../move";
import * as compare from "../compare";

function isElementOfType<K extends ElementType>(
  element: LadderElement,
  type: K
): element is Extract<LadderElement, { type: K }> {
  return element.type === type;
}

function entry<K extends ElementType>(mod: {
  definition: ElementDefinition<K>;
  Glyph: ElementGlyph<K>;
}): RegistryEntry {
  const { definition } = mod;
  return {
    def: {
      kind: definition.kind,
      category: definition.category,
      label: definition.label,
      shortLabel: definition.shortLabel,
      help: definition.help,
      cellClass: definition.cellClass,
      create: definition.create,
      topLabel: (element, fmt) =>
        isElementOfType(element, definition.kind) ? definition.topLabel(element, fmt) : "",
      bottomLabel: (element, fmt) =>
        isElementOfType(element, definition.kind) ? definition.bottomLabel(element, fmt) : "",
    },
    Glyph: mod.Glyph as RegistryEntry["Glyph"],
  };
}

/** All ladder instructions keyed by AST type. */
export const ELEMENT_REGISTRY: Record<ElementType, RegistryEntry> = {
  contact_no: entry(contactNo),
  contact_nc: entry(contactNc),
  contact_rising: entry(contactRising),
  contact_falling: entry(contactFalling),
  coil: entry(coil),
  coil_negated: entry(coilNegated),
  coil_set: entry(coilSet),
  coil_reset: entry(coilReset),
  wire: entry(wire),
  ton: entry(ton),
  tof: entry(tof),
  rto: entry(rto),
  ctu: entry(ctu),
  ctd: entry(ctd),
  math: entry(math),
  move: entry(move),
  compare: entry(compare),
};

/** Display order in palette (groups defined below). */
const PALETTE_ORDER: ElementType[] = [
  "contact_no",
  "contact_nc",
  "contact_rising",
  "contact_falling",
  "coil",
  "coil_negated",
  "coil_set",
  "coil_reset",
  "wire",
  "ton",
  "tof",
  "rto",
  "ctu",
  "ctd",
  "math",
  "move",
  "compare",
];

export function getDefinition(type: ElementType): ElementDefinition {
  return ELEMENT_REGISTRY[type].def;
}

export function getRegistryEntry(type: ElementType): RegistryEntry {
  return ELEMENT_REGISTRY[type];
}

/** Factory used by the PLC store / palette drop. */
export function createElement(kind: PaletteKind): LadderElement {
  if (kind === "or_branch") {
    return ELEMENT_REGISTRY.wire.def.create();
  }
  return ELEMENT_REGISTRY[kind as ElementType].def.create();
}

export function isBitElement(type: ElementType): boolean {
  const c = ELEMENT_REGISTRY[type].def.cellClass;
  return c === "contact" || c === "coil" || c === "edge";
}

export function isCoilType(type: ElementType): boolean {
  return ELEMENT_REGISTRY[type].def.cellClass === "coil";
}

export function isFbType(type: ElementType): boolean {
  return ELEMENT_REGISTRY[type].def.cellClass === "fb";
}

export function isTimerType(type: ElementType): boolean {
  return ELEMENT_REGISTRY[type].def.category === "timer";
}

export function isCounterType(type: ElementType): boolean {
  return ELEMENT_REGISTRY[type].def.category === "counter";
}

export interface PaletteGroup {
  title: string;
  items: {
    kind: PaletteKind;
    label: string;
    type: ElementType;
  }[];
}

function itemsFor(types: ElementType[]): PaletteGroup["items"] {
  return types.map((type) => ({
    kind: type as PaletteKind,
    label: ELEMENT_REGISTRY[type].def.label,
    type,
  }));
}

/** Palette groups (TIA-style sections). */
export function paletteGroups(): PaletteGroup[] {
  return [
    {
      title: "Bit logic",
      items: [
        ...itemsFor([
          "contact_no",
          "contact_nc",
          "contact_rising",
          "contact_falling",
          "coil",
          "coil_negated",
          "coil_set",
          "coil_reset",
          "wire",
        ]),
        { kind: "or_branch", label: "Parallel OR", type: "wire" },
      ],
    },
    {
      title: "Timers / counters",
      items: itemsFor(["ton", "tof", "rto", "ctu", "ctd"]),
    },
    {
      title: "Math / move / cmp",
      items: itemsFor(["math", "move", "compare"]),
    },
  ];
}

export { PALETTE_ORDER };
