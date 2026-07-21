# PLC core (Rust)

| Module | Responsibility |
|--------|----------------|
| `memory.rs` | Process image (I/Q/M/R/MR/IW) + compact UI snapshot |
| `compiler.rs` | Ladder AST + all element types + bytecode |
| `engine.rs` | Scan cycle, edge memory, SET/RESET, timers |
| `modbus.rs` | TCP slave start/stop/port |
| `modbus_map.rs` | Address visibility map |
| `symbols.rs` | PLC tag table |

Element **semantics** live in `compiler.rs` (`LadderElement` / `Instruction`).  
Element **UI** lives in `src/features/ladder/components/elements/` (one Svelte file each).
