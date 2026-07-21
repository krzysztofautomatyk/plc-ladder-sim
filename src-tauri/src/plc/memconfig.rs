//! =============================================================================
//! Memory allocation — configurable address-space sizes / ranges.
//!
//! Lets the operator size each PLC memory area (inputs, outputs, markers,
//! data registers 16/32-bit, internal registers, timers, counters) like a
//! classic PLC "Memory Allocation" editor. Counts are validated against the
//! physical process-image maxima; the frontend uses them to show ranges and to
//! validate that ladder addresses stay inside the allocated space.
//! =============================================================================

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::memory::{
    COIL_COUNT, DISCRETE_INPUT_COUNT, HOLDING_REGISTER_COUNT, MEMORY_BIT_COUNT, MEMORY_WORD_COUNT,
};

/// Hard caps for timer / counter instances (function-block indices).
pub const TIMER_MAX: u16 = 256;
pub const COUNTER_MAX: u16 = 256;

/// Per-area element counts (how many of each area are allocated / usable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Discrete inputs I0..I(n-1).
    pub inputs: u16,
    /// Coils / outputs Q0..Q(n-1).
    pub outputs: u16,
    /// Internal marker bits M0..M(n-1) — Modbus only via explicit matrix rule.
    pub markers: u16,
    /// 16-bit data registers R0..R(n-1) — Modbus holding (4x). User data;
    /// engine publishes T/C status at R2048+ / R3072+ (outside default data16).
    pub data16: u16,
    /// Reserved pool for future 32-bit RD (2 words each above data16). Not yet
    /// ladder-addressable as RD prefix.
    pub data32: u16,
    /// 16-bit internal registers MR0..MR(n-1) — Modbus only via explicit matrix rule.
    pub internal16: u16,
    /// Timer instances T0..T(n-1).
    pub timers: u16,
    /// Counter instances C0..C(n-1).
    pub counters: u16,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            inputs: 128,
            outputs: 128,
            markers: 1024,
            data16: 1024,
            data32: 0,
            internal16: 1024,
            timers: 64,
            counters: 64,
        }
    }
}

impl MemoryConfig {
    /// Physical upper bounds for each field (from the process-image pool sizes).
    pub fn limits() -> Self {
        Self {
            inputs: DISCRETE_INPUT_COUNT as u16,
            outputs: COIL_COUNT as u16,
            markers: MEMORY_BIT_COUNT as u16,
            data16: HOLDING_REGISTER_COUNT as u16,
            data32: (HOLDING_REGISTER_COUNT / 2) as u16,
            internal16: MEMORY_WORD_COUNT as u16,
            timers: TIMER_MAX,
            counters: COUNTER_MAX,
        }
    }

    /// Validate against the physical maxima and the shared R-pool budget.
    pub fn validate(&self) -> Result<(), String> {
        let lim = Self::limits();
        let check = |name: &str, v: u16, max: u16| -> Result<(), String> {
            if v > max {
                Err(format!("{name} {v} exceeds maximum {max}"))
            } else {
                Ok(())
            }
        };
        check("inputs", self.inputs, lim.inputs)?;
        check("outputs", self.outputs, lim.outputs)?;
        check("markers", self.markers, lim.markers)?;
        check("data16", self.data16, lim.data16)?;
        check("data32", self.data32, lim.data32)?;
        check("internal16", self.internal16, lim.internal16)?;
        check("timers", self.timers, lim.timers)?;
        check("counters", self.counters, lim.counters)?;

        // 16-bit and 32-bit data registers share the same physical R pool;
        // each 32-bit register occupies two words above the 16-bit range.
        let words_used = self.data16 as usize + 2 * self.data32 as usize;
        if words_used > HOLDING_REGISTER_COUNT {
            return Err(format!(
                "R pool overflow: {} 16-bit + {}×2 32-bit words = {} exceeds {}",
                self.data16, self.data32, words_used, HOLDING_REGISTER_COUNT
            ));
        }
        Ok(())
    }
}

/// Config paired with the physical maxima (sent to the frontend in one shot).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct MemoryConfigInfo {
    pub config: MemoryConfig,
    pub limits: MemoryConfig,
    /// Total words in the shared 16/32-bit R (holding) pool.
    pub register_pool: u16,
}

/// Thread-safe holder for the current allocation.
#[derive(Debug)]
pub struct MemoryConfigStore {
    inner: Mutex<MemoryConfig>,
}

impl MemoryConfigStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(MemoryConfig::default()),
        })
    }

    pub fn get(&self) -> MemoryConfig {
        *self.inner.lock()
    }

    pub fn info(&self) -> MemoryConfigInfo {
        MemoryConfigInfo {
            config: self.get(),
            limits: MemoryConfig::limits(),
            register_pool: HOLDING_REGISTER_COUNT as u16,
        }
    }

    /// Validate and store a new allocation.
    pub fn set(&self, config: MemoryConfig) -> Result<(), String> {
        config.validate()?;
        *self.inner.lock() = config;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        assert!(MemoryConfig::default().validate().is_ok());
    }

    #[test]
    fn rejects_over_maximum() {
        let c = MemoryConfig {
            markers: MEMORY_BIT_COUNT as u16 + 1,
            ..Default::default()
        };
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_register_pool_overflow() {
        let c = MemoryConfig {
            data16: HOLDING_REGISTER_COUNT as u16,
            data32: 1, // needs 2 extra words → overflow
            ..Default::default()
        };
        assert!(c.validate().is_err());
    }

    #[test]
    fn store_roundtrip_and_reject() {
        let store = MemoryConfigStore::new();
        let ok = MemoryConfig {
            timers: 200,
            ..Default::default()
        };
        assert!(store.set(ok).is_ok());
        assert_eq!(store.get().timers, 200);

        let bad = MemoryConfig {
            timers: TIMER_MAX + 1,
            ..Default::default()
        };
        assert!(store.set(bad).is_err());
        assert_eq!(store.get().timers, 200, "rejected config must not apply");
    }
}
