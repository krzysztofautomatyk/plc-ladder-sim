import type { Address, MemorySnapshot } from "../../../shared/lib/types";
import { COUNTER_HR_BASE, TIMER_HR_BASE } from "./addressFormat";

/** Live timer ET (ms) for FB online display. */
export function readTimerEt(mem: MemorySnapshot | null | undefined, timerIndex: number): number {
  if (!mem) return 0;
  if (mem.timer_et && timerIndex < mem.timer_et.length) {
    return mem.timer_et[timerIndex] ?? 0;
  }
  // Fallback: holding bank (only if present in compact slice).
  return mem.holding_registers[TIMER_HR_BASE + 2 * timerIndex] ?? 0;
}

export function readTimerQ(mem: MemorySnapshot | null | undefined, timerIndex: number): boolean {
  if (!mem) return false;
  if (mem.timer_q && timerIndex < mem.timer_q.length) {
    return Boolean(mem.timer_q[timerIndex]);
  }
  return (mem.holding_registers[TIMER_HR_BASE + 2 * timerIndex + 1] ?? 0) !== 0;
}

export function readCounterCv(mem: MemorySnapshot | null | undefined, counterIndex: number): number {
  if (!mem) return 0;
  if (mem.counter_cv && counterIndex < mem.counter_cv.length) {
    return mem.counter_cv[counterIndex] ?? 0;
  }
  return mem.holding_registers[COUNTER_HR_BASE + 2 * counterIndex] ?? 0;
}

export function readCounterQ(mem: MemorySnapshot | null | undefined, counterIndex: number): boolean {
  if (!mem) return false;
  if (mem.counter_q && counterIndex < mem.counter_q.length) {
    return Boolean(mem.counter_q[counterIndex]);
  }
  return (mem.holding_registers[COUNTER_HR_BASE + 2 * counterIndex + 1] ?? 0) !== 0;
}

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
      // Prefer dedicated T/C images when address is in a bank (compact HR may omit them).
      if (idx >= TIMER_HR_BASE && idx < TIMER_HR_BASE + 512) {
        const off = idx - TIMER_HR_BASE;
        const ti = Math.floor(off / 2);
        if (off % 2 === 0) return readTimerEt(mem, ti) !== 0;
        return readTimerQ(mem, ti);
      }
      if (idx >= COUNTER_HR_BASE && idx < COUNTER_HR_BASE + 512) {
        const off = idx - COUNTER_HR_BASE;
        const ci = Math.floor(off / 2);
        if (off % 2 === 0) return readCounterCv(mem, ci) !== 0;
        return readCounterQ(mem, ci);
      }
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

/**
 * Read a numeric process-image value for online FB display (MOVE/math/cmp).
 * Bits → 0/1; words → u16. Timer/counter banks use dedicated snapshot fields.
 */
export function readMemoryValue(
  mem: MemorySnapshot | null | undefined,
  addr: Address | null | undefined
): number {
  if (!mem || !addr) return 0;
  const idx = addr.index;
  switch (addr.area) {
    case "coil":
      return mem.coils[idx] ? 1 : 0;
    case "discrete":
      return mem.discrete_inputs[idx] ? 1 : 0;
    case "memory_bit":
      return mem.memory_bits?.[idx] ? 1 : 0;
    case "holding": {
      if (idx >= TIMER_HR_BASE && idx < TIMER_HR_BASE + 512) {
        const off = idx - TIMER_HR_BASE;
        const ti = Math.floor(off / 2);
        if (off % 2 === 0) return readTimerEt(mem, ti);
        return readTimerQ(mem, ti) ? 1 : 0;
      }
      if (idx >= COUNTER_HR_BASE && idx < COUNTER_HR_BASE + 512) {
        const off = idx - COUNTER_HR_BASE;
        const ci = Math.floor(off / 2);
        if (off % 2 === 0) return readCounterCv(mem, ci);
        return readCounterQ(mem, ci) ? 1 : 0;
      }
      const v = mem.holding_registers[idx] ?? 0;
      if (addr.bit != null && addr.bit >= 0) {
        return (v >> (addr.bit & 15)) & 1;
      }
      return v & 0xffff;
    }
    case "input_reg": {
      const v = mem.input_registers[idx] ?? 0;
      if (addr.bit != null && addr.bit >= 0) {
        return (v >> (addr.bit & 15)) & 1;
      }
      return v & 0xffff;
    }
    case "memory_word": {
      const v = mem.memory_words?.[idx] ?? 0;
      if (addr.bit != null && addr.bit >= 0) {
        return (v >> (addr.bit & 15)) & 1;
      }
      return v & 0xffff;
    }
    default:
      return 0;
  }
}

/** Format address + live value for FB parameter rows, e.g. `R40=123`. */
export function formatLiveOperand(
  mem: MemorySnapshot | null | undefined,
  addr: Address | null | undefined,
  formatAddress: (a: Address | null | undefined) => string
): string {
  if (!addr) return "—";
  const label = formatAddress(addr);
  const val = readMemoryValue(mem, addr);
  return `${label}=${val}`;
}

/** Split address / live value for TIA-style FB rows (addr left, big value badge). */
export function formatLiveParts(
  mem: MemorySnapshot | null | undefined,
  addr: Address | null | undefined,
  formatAddress: (a: Address | null | undefined) => string
): { addr: string; val: string } {
  if (!addr) return { addr: "—", val: "" };
  return {
    addr: formatAddress(addr),
    val: String(readMemoryValue(mem, addr)),
  };
}

/** True if this ladder element has a primary address we can monitor. */
export function elementAddress(el: { address?: Address } | { type: string }): Address | null {
  if ("address" in el && el.address) return el.address;
  return null;
}

/**
 * Best address for symbolic labels on the ladder canvas.
 * Prefer the operand operators care about (coil/contact addr, MOVE dest, etc.).
 */
export function labelAddress(el: {
  type: string;
  address?: Address;
  done_address?: Address | null;
  source?: Address;
  dest?: Address;
  a?: Address;
}): Address | null {
  if ("address" in el && el.address) return el.address;
  if (el.type === "move" && el.dest) return el.dest;
  if ((el.type === "math" || el.type === "compare") && el.a) return el.a;
  if (el.done_address) return el.done_address;
  if (el.dest) return el.dest;
  return null;
}
