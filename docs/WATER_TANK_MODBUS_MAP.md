# Water Tank — Mapa rejestrów Modbus (HR 100…150)

**Slave:** `127.0.0.1:5020`  
**FC03** odczyt holding · **FC06/16** zapis (gdy *Allow SCADA writes*)  
**Aplikacja:** toolbar → **Mapa rejestrów** · drzewo → *Device config → Mapa rejestrów*  
**Kod:** `src/shared/lib/waterTankModbusMap.ts` · JSON: `programs/water_tank_modbus_map.json`

Cały obraz stacji dla SCADA jest w **jednym bloku holding 100–150** (bity w HR100–103).

### Zapis z SCADA (ważne)

| Problem | Przyczyna | Rozwiązanie |
|---------|-----------|-------------|
| **Exception 0x04 ServerDeviceFailure** | Globalny gate zapisu wyłączony | **Mapa rejestrów** → ☑ *Allow SCADA writes* albo *Zastosuj mapę + włącz zapis* · albo **Modbus TCP** → Allow SCADA writes |
| **Exception 0x02 Illegal Data Address** | Reguła write-protected (RO) | Zapisuj tylko HR z kolumną **R/W** (np. 104–106, 108–110) |
| Brak połączenia | Slave OFF | Start Modbus w aplikacji |

Przycisk **Water tank** automatycznie: mapa HR100–150 + **writes ON** + start slave.

**Uwaga adresów:** **HR109 = SP_P1_ON (PLC R106, seed 700)** — to nie jest PLC R109 (stała C0).  
Żeby zmienić próg P1 z 700 → 650: **FC06 HR109 = 650** (przy writes ON).

---

## Tabela HR 100–150

| HR | Symbol | PLC | R/W | Zawartość |
|----|--------|-----|-----|-----------|
| **100** | DI_PACK | I0…I15 | R | Wejścia spakowane (bit0=I0 … bit9=I9) |
| **101** | DO_PACK | Q0…Q15 | R | Wyjścia spakowane (bit0=Q0 … bit4=Q4) |
| **102** | M_LO | M0…M15 | R | Markery sterowania |
| **103** | M_HI | M16…M31 | R | Markery sim (ONE/BOTH/ANY/DRAIN) |
| **104** | LEVEL_cm | R100 | R/W\* | Poziom 0…1000 cm |
| **105** | K_x100 | R101 | R/W | Napływ ×100 |
| **106** | FILL_STEP | R102 | R/W | cm / tick 200 ms |
| **107** | PUMP_STEP | R103 | R | Wydajność 1 pompy / tick |
| **108** | SP_STOP | R105 | R/W | Stop 200 cm |
| **109** | SP_P1_ON | R106 | R/W | Start P1 700 cm |
| **110** | SP_P2_ON | R107 | R/W | P2 800 cm |
| **111** | CAP | R112 | R | Suma wydajności pomp |
| **112** | DRAIN | R113 | R | CAP−FILL |
| **113** | FILL_NET | R114 | R | FILL−CAP |
| **114** | P1_HH | R120 | R | Czas P1 h |
| **115** | P1_MM | R121 | R | Czas P1 min |
| **116** | P1_SS | R122 | R | Czas P1 s |
| **117** | P2_HH | R123 | R | Czas P2 h |
| **118** | P2_MM | R124 | R | Czas P2 min |
| **119** | P2_SS | R125 | R | Czas P2 s |
| **120** | P1_STARTS | R126 | R | Starty P1 |
| **121** | P2_STARTS | R127 | R | Starty P2 |
| **122…150** | RES_* | — | R | Rezerwa |

\* Przy RUN + SIM_EN (I0) program nadpisuje poziom.

### Bity HR100 (DI_PACK)

| Bit | Tag | Znaczenie |
|-----|-----|-----------|
| 0 | I0 | SIM_EN |
| 1 | I1 | FLT_LO |
| 2 | I2 | FLT_HI |
| 3 | I3 | P1_FAULT |
| 4 | I4 | P2_FAULT |
| 5 | I5 | P1_LOCK |
| 6 | I6 | P2_LOCK |
| 7 | I7 | RST_STAT |
| 8 | I8 | MAN_P1 |
| 9 | I9 | MAN_P2 |
| 10–15 | — | rezerwa |

### Bity HR101 (DO_PACK)

| Bit | Tag | Znaczenie |
|-----|-----|-----------|
| 0 | Q0 | P1_RUN |
| 1 | Q1 | P2_RUN |
| 2 | Q2 | ALM_HI |
| 3 | Q3 | ALM_FLT |
| 4 | Q4 | ALM_FAIL |

### Bity HR102 (M_LO)

| Bit | Tag | Znaczenie |
|-----|-----|-----------|
| 0 | M0 | TICK |
| 2 | M2 | DEMAND |
| 3 | M3 | JOIN_P2 |
| 4 | M4 | P1_OK |
| 5 | M5 | P2_OK |
| 6 | M6 | P1_CMD |
| 7 | M7 | P2_LEAD |
| 8 | M8 | P2_CMD |
| 10 | M10 | V_FLT_LO |
| 11 | M11 | V_FLT_HI |

### Bity HR103 (M_HI)

| Bit | Tag | Znaczenie |
|-----|-----|-----------|
| 4 | M20 | ONE_P |
| 5 | M21 | BOTH |
| 6 | M22 | ANY_P |
| 9 | M25 | DRAIN_REGIME |

---

## SCADA — jeden odczyt

```text
FC03 start=100 quantity=22   → HR100…HR121 (cała stacja)
```

```python
from pymodbus.client import ModbusTcpClient
c = ModbusTcpClient("127.0.0.1", port=5020)
c.connect()
r = c.read_holding_registers(100, 22)
level, k = r.registers[4], r.registers[5]   # HR104, HR105
di, do = r.registers[0], r.registers[1]     # packs
demand = bool(r.registers[2] & (1 << 2))    # M2 in HR102
print(level, k, di, do, demand)
c.close()
```

---

## Ładowanie

1. Toolbar **Water tank** — program + seed + ta mapa automatycznie.  
2. Lub **Mapa rejestrów** → *Zastosuj mapę do Modbus*.  
3. **Modbus TCP** → Allow SCADA writes (tylko HR R/W).
