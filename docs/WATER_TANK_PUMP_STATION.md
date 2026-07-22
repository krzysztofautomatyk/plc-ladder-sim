# Water Tank Dual-Pump Station — LAD Specification

**Project:** `Water_Tank_Dual_Pump` v1.0.0  
**Standard:** IEC 61131-3 Ladder Diagram (subset implemented by PLC Ladder Simulator Pro)  
**Domain:** Municipal / industrial wet-well pump-out station with continuous inflow  
**Load in app:** toolbar → **Water tank**  
**Modbus map:** `docs/WATER_TANK_MODBUS_MAP.md` · auto-applied on load · `programs/water_tank_modbus_map.json`

---

## 1. Process description

A collection chamber (wet-well) receives continuous **inflow**. Two submersible pumps empty the chamber to the force main. Control is fully automatic from a **hydrostatic level probe** with optional **float backup**, plus operator **locks**, **fault inputs**, **manual force**, and **statistics** (run time + start counts).

A dedicated **simulation section** in the same OB1 image integrates a discrete tank model so the station can be demonstrated without field I/O.

```
                    ┌── inflow (continuous) ──┐
                    ▼                         │
              ╔═══════════════╗               │
   FLT_HI ────╢  SP_MAX       ║               │
   SP_P2  ────╢  (2nd pump)   ║   P1 ══╗      │
   FLT_LO ────╢  SP_MIN       ║   P2 ══╬══ outfall
              ╚═══════════════╝        ║
                 LEVEL (probe)         ▼
```

---

## 2. Inflow factor **K** (core simulation parameter)

| Register | Symbol | Scale | Meaning |
|----------|--------|-------|---------|
| **R101** | `K_x100` | ×100 | **K = napływ / wydajność_jednej_pompy** |

### Interpretation (engineering)

| K (eng.) | R101 | One pump running | Two pumps running |
|----------|------|------------------|-------------------|
| **0.50** | **50** | Outflow > inflow → level **falls** (about ½ as “fast” as free fill would rise, net of residual inflow) | Falls faster |
| **1.00** | **100** | Outflow ≈ inflow → level roughly **holds** | Falls |
| **1.50** | **150** | Outflow < inflow → level **still rises** | Outflow > inflow → level **falls** |

**Defaults after *Water tank* load:** `R101 = 150` (stress case: one pump insufficient, two pumps recover).

### Discrete model (each 200 ms sim tick while **I0** = SIM_EN)

```
FILL_STEP  = R102          (default 10 level units / tick)
PUMP_STEP  = FILL_STEP × 100 / K_x100     → R103
CAP        = n_pumps × PUMP_STEP          → R112   (n = 0, 1 or 2)
if CAP ≥ FILL:  LEVEL −= (CAP − FILL)     # draining regime
else:           LEVEL += (FILL − CAP)     # filling regime
LEVEL clamped to 0 … 1000
```

**Level scale:** `R100` **0…1000** ⇔ **0.0…100.0 %**

---

## 3. Setpoints (hydrostatic probe) — **cm**

| Reg | Symbol | Default | Function |
|-----|--------|---------|----------|
| **R105** | `SP_STOP` | **200 cm** | **RESET DEMAND** — all pumps OFF |
| **R106** | `SP_P1_ON` | **700 cm** | **SET DEMAND** — pump 1 ON |
| **R107** | `SP_P2_ON` | **800 cm** | Pump 2 joins (assist) while DEMAND latched |

### Hysteresis (empty cycle)

```
level rises (inflow) ──► ≥700 cm: P1 starts (DEMAND SET)
                         ≥800 cm: P2 joins
level falls (pumps)  ──► ≤200 cm: DEMAND RESET, P1+P2 stop
```

Pumps **do not** drop out at 700 cm — they keep running until **200 cm**.

---

## 4. Discrete I/O map

### Inputs (force in **Watch** panel)

| Addr | Symbol | Type | Description |
|------|--------|------|-------------|
| **I0** | `SIM_EN` | BOOL | Enable level simulation section |
| **I1** | `FLT_LO` | BOOL | Hardwired low float (TRUE = water above low float / OK to run) |
| **I2** | `FLT_HI` | BOOL | Hardwired high float (TRUE = high water) |
| **I3** | `P1_FAULT` | BOOL | Pump 1 fault (thermal / overload / VFD trip) |
| **I4** | `P2_FAULT` | BOOL | Pump 2 fault |
| **I5** | `P1_LOCK` | BOOL | Operator lock — **blocks** pump 1 |
| **I6** | `P2_LOCK` | BOOL | Operator lock — **blocks** pump 2 |
| **I7** | `RST_STATS` | BOOL | Reset run-time RTO + start counters |
| **I8** | `MAN_P1` | BOOL | Manual force request pump 1 (still needs DEMAND path / OK) |
| **I9** | `MAN_P2` | BOOL | Manual force request pump 2 |

### Outputs

| Addr | Symbol | Description |
|------|--------|-------------|
| **Q0** | `P1_RUN` | Pump 1 contactor / run command |
| **Q1** | `P2_RUN` | Pump 2 contactor / run command |
| **Q2** | `ALM_HI` | High level alarm |
| **Q3** | `ALM_FAULT` | Any pump fault |
| **Q4** | `ALM_BOTH_DOWN` | Demand present but **no** pump available |

---

## 5. Internal markers (M)

| Addr | Symbol | Description |
|------|--------|-------------|
| M0 | `TICK` | 200 ms free-running simulation pulse |
| M2 | `DEMAND` | Pump-out demand (seal-in) |
| M3 | `NEED_P2` | Level ≥ SP_P2 — need lag pump |
| M4 | `P1_OK` | Pump 1 available (¬fault ∧ ¬lock) |
| M5 | `P2_OK` | Pump 2 available |
| M6 | `P1_CMD` | Internal command P1 |
| M7 | `P2_IS_LEAD` | Lead role given to P2 (when P1 not OK) |
| M8 | `P2_CMD` | Internal command P2 |
| M10 | `V_FLT_LO` | Virtual float LO from probe |
| M11 | `V_FLT_HI` | Virtual float HI from probe |
| M20 | `ONE_P` | Exactly one pump running |
| M21 | `TWO_P` | Both pumps running |
| M22 | `ANY_P` | At least one pump running |
| M25 | `DRAIN_REGIME` | CAP ≥ FILL (net emptying) |

---

## 6. Holding registers (R)

| Reg | Symbol | R/W | Description |
|-----|--------|-----|-------------|
| **R100** | `LEVEL` | R/W | Process level 0…1000 |
| **R101** | `K_x100` | R/W | Inflow factor ×100 |
| **R102** | `FILL_STEP` | R/W | Inflow units per sim tick |
| **R103** | `PUMP_STEP` | R | Computed single-pump capacity / tick |
| **R104** | `TMP` | R | Scratch (level+net) |
| **R105** | `SP_MIN` | R/W | Stop setpoint |
| **R106** | `SP_MAX` | R/W | High / start setpoint |
| **R107** | `SP_P2` | R/W | Second-pump setpoint |
| **R108** | `C100` | R/W | Constant 100 |
| **R109** | `C0` | R/W | Constant 0 |
| **R110** | `C1000` | R/W | Constant 1000 (100 %) |
| **R111** | `TMP2` | R | Scratch (FILL×100) |
| **R112** | `CAP` | R | Total pump capacity this tick |
| **R113** | `DRAIN` | R | CAP − FILL |
| **R114** | `FILL_NET` | R | FILL − CAP |
| **R120** | `P1_RUN_MS` | R | Pump 1 accumulated run time (ms, from RTO T0) |
| **R121** | `P2_RUN_MS` | R | Pump 2 accumulated run time (ms, from RTO T1) |
| **R122** | `P1_STARTS` | R | Pump 1 start count (from CTU C0) |
| **R123** | `P2_STARTS` | R | Pump 2 start count (from CTU C1) |

**Live timer/counter banks (also on FB glyphs):**

| Address | Maps to |
|---------|---------|
| `TV0` / HR 2048 | RTO T0 ET (P1) |
| `TV1` / HR 2050 | RTO T1 ET (P2) |
| `CV0` / HR 3072 | CTU C0 (P1 starts) |
| `CV1` / HR 3074 | CTU C1 (P2 starts) |

---

## 7. Control strategy (lead / lag / failover)

1. **DEMAND** seals on high level (virtual or hardwired), on level ≥ SP_P2, or existing seal; holds while level remains above SP_MIN (virtual LO or hardwired I1).
2. **P1_OK / P2_OK** clear on fault or lock.
3. If P1 is not OK → **P2 becomes lead** (M7).
4. **P1_CMD** when demand and P1 OK and any of: P1 lead, NEED_P2, P2 unavailable, MAN_P1.
5. **P2_CMD** when demand and P2 OK and any of: P2 lead, NEED_P2, P1 unavailable, **P1_FAULT**, MAN_P2.
6. Therefore: **running P1 + P1_FAULT** immediately enables P2 (failover) if P2 is OK and demand remains.
7. **Locks** (I5/I6) force the unit unavailable without raising a fault lamp by themselves.

### Scenario matrix (K = 1.5, defaults)

| Condition | Expected behaviour |
|-----------|-------------------|
| Idle, level rising | No pumps; LEVEL climbs on sim ticks |
| Level ≥ 85 % | DEMAND, ALM_HI; P1 (lead) starts |
| Level ≥ 70 % while demand | NEED_P2 → both pumps if available |
| P1 faults while running | P2 takes over (failover) |
| Both locked/faulted + demand | Q4 station-fail alarm |
| Level returns ≤ 15 % | DEMAND drops; pumps stop |

### Scenario K = 0.5

One pump capacity is high vs inflow → level falls with a single pump; second pump rarely needed except for high-level speed-up or failover.

---

## 8. Network map (LAD sections)

| Tag in comment | Networks | Responsibility |
|----------------|----------|----------------|
| `[SIM]` | Clock, K math, CAP, level integrate | Process simulation |
| `[SENS]` | Virtual floats, HI alarm | Instrumentation |
| `[CTRL]` | DEMAND, NEED_P2 | Process demand |
| `[P1]` / `[P2]` / `[LEAD]` / `[OUT]` | Availability, commands, contactors | Motor control |
| `[ALM]` | Fault & station fail | Alarms |
| `[STAT]` | RTO, CTU, publish to R12x | KPI / maintenance |

---

## 9. How to run (lab procedure)

1. Open app → **Water tank** (loads LAD + seeds R/I).
2. Confirm **I0** SIM_EN = 1 (Watch).
3. Set **R101**:
   - `50` → easy empty with one pump  
   - `150` → need two pumps to pull level down  
4. **RUN**. Watch **R100 LEVEL**, TON/CTU/MOVE live values, Q0/Q1.
5. Force **I3** (P1 fault) while P1 runs → Q1 should pick up if demand remains.
6. Force **I5** (P1 lock) → P1 unavailable; P2 may lead.
7. Pulse **I7** to clear run-time and start counters.

**Optional hard floats:** drive I1/I2 from field; virtual M10/M11 still track the probe.

---

## 10. Maintenance KPIs

| KPI | Reg | Symbol | Reset |
|-----|-----|--------|-------|
| Pump 1 hours | **R120** | `P1_HH` | I7 |
| Pump 1 minutes | **R121** | `P1_MM` | I7 |
| Pump 1 seconds | **R122** | `P1_SS` | I7 |
| Pump 2 hours | **R123** | `P2_HH` | I7 |
| Pump 2 minutes | **R124** | `P2_MM` | I7 |
| Pump 2 seconds | **R125** | `P2_SS` | I7 |
| Pump 1 starts | **R126** | `P1_STARTS` | I7 |
| Pump 2 starts | **R127** | `P2_STARTS` | I7 |

Clocks tick once per second while the corresponding pump contactor is ON (`Q0` / `Q1`). Rollover: SS→MM at 60, MM→HH at 60.

---

## 11. Safety / training notes

- Training simulator — **not** a certified safety PLC.
- Locks are soft interlocks in LAD; field E-stop / hardwired safeties are out of scope.
- Integer math only: keep **K_x100 ≥ 1** to avoid DIV-by-zero skip (PUMP_STEP freezes).
- After **Reset I/O**, re-press **Water tank** to re-seed constants (R108–R110, setpoints, K).

---

## 12. File map

| Asset | Path |
|-------|------|
| LAD factory (TypeScript) | `src/shared/lib/waterTankProgram.ts` |
| This specification | `docs/WATER_TANK_PUMP_STATION.md` |
| Load entry | `plc.loadWaterTank()` · toolbar **Water tank** |

---

*Prepared as an enterprise teaching pack for dual-pump wet-well control with transparent process simulation and full tag discipline.*
