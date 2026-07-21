import type { Address, MemorySnapshot } from "../../../shared/lib/types";

/** Read a boolean from the live process image for ladder online display. */
export function readMemoryBit(mem: MemorySnapshot | null | undefined, addr: Address): boolean {
  if (!mem) return false;
  const idx = addr.index;
  switch (addr.area) {
    case "coil":
      return Boolean(mem.coils[idx]);
    case "discrete":
      return Boolean(mem.discrete_inputs[idx]);
    case "holding": {
      const v = mem.holding_registers[idx] ?? 0;
      if (addr.bit != null && addr.bit >= 0) {
        return ((v >> (addr.bit & 15)) & 1) === 1;
      }
      return v !== 0;
    }
    case "input_reg": {
      const v = mem.input_registers[idx] ?? 0;
      if (addr.bit != null && addr.bit >= 0) {
        return ((v >> (addr.bit & 15)) & 1) === 1;
      }
      return v !== 0;
    }
    case "memory_bit":
      return Boolean(mem.memory_bits?.[idx]);
    case "memory_word": {
      const v = mem.memory_words?.[idx] ?? 0;
      if (addr.bit != null && addr.bit >= 0) {
        return ((v >> (addr.bit & 15)) & 1) === 1;
      }
      return v !== 0;
    }
    default:
      return false;
  }
}

/** True if this ladder element has a primary address we can monitor. */
export function elementAddress(el: { address?: Address } | { type: string }): Address | null {
  if ("address" in el && el.address) return el.address;
  return null;
}
