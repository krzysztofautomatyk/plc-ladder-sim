//! =============================================================================
//! In-memory log buffer + tracing `Layer`.
//!
//! A packaged Tauri GUI app has no attached console, so `tracing` output that
//! only goes to stdout is invisible to operators. This module captures every
//! tracing event into a bounded ring buffer that the frontend "Logs" tab can
//! read via the `get_logs` / `clear_logs` commands — giving full visibility into
//! the scan engine, Modbus TCP slave (connections, requests, exceptions) and the
//! audit chain without a terminal.
//! =============================================================================

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Serialize;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// Maximum number of retained log lines (oldest are dropped first).
const CAPACITY: usize = 5_000;

/// A single captured log line, serialized to the frontend Logs tab.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// Monotonic sequence number (stable ordering / de-dup on the client).
    pub seq: u64,
    /// Local wall-clock timestamp, `YYYY-MM-DD HH:MM:SS.mmm`.
    pub ts: String,
    /// Log level: `error` | `warn` | `info` | `debug` | `trace`.
    pub level: String,
    /// Event target (module path or explicit `target:`).
    pub target: String,
    /// The human-readable message.
    pub message: String,
    /// Remaining structured fields rendered as `key=value` pairs.
    pub fields: String,
}

struct LogStore {
    buf: Mutex<VecDeque<LogEntry>>,
    seq: AtomicU64,
}

impl LogStore {
    fn new() -> Self {
        Self {
            buf: Mutex::new(VecDeque::with_capacity(CAPACITY)),
            seq: AtomicU64::new(0),
        }
    }

    fn push(&self, level: &str, target: &str, message: String, fields: String) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let ts = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S%.3f")
            .to_string();
        let entry = LogEntry {
            seq,
            ts,
            level: level.to_string(),
            target: target.to_string(),
            message,
            fields,
        };
        let mut buf = self.buf.lock();
        if buf.len() >= CAPACITY {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}

static LOG_STORE: Lazy<LogStore> = Lazy::new(LogStore::new);

fn level_rank(level: &str) -> u8 {
    match level {
        "error" => 4,
        "warn" => 3,
        "info" => 2,
        "debug" => 1,
        _ => 0, // trace / unknown
    }
}

fn level_str(level: &Level) -> &'static str {
    match *level {
        Level::ERROR => "error",
        Level::WARN => "warn",
        Level::INFO => "info",
        Level::DEBUG => "debug",
        Level::TRACE => "trace",
    }
}

/// Return the most recent log lines (chronological, oldest first), keeping only
/// entries at or above `min_level` and capping the result at `limit`.
pub fn snapshot(limit: usize, min_level: &str) -> Vec<LogEntry> {
    let min = level_rank(min_level);
    let buf = LOG_STORE.buf.lock();
    let mut out: Vec<LogEntry> = buf
        .iter()
        .filter(|e| level_rank(&e.level) >= min)
        .cloned()
        .collect();
    if out.len() > limit {
        out.drain(0..out.len() - limit);
    }
    out
}

/// Drop all buffered log lines. Returns the number of lines removed.
pub fn clear() -> usize {
    let mut buf = LOG_STORE.buf.lock();
    let n = buf.len();
    buf.clear();
    n
}

/// Field visitor extracting the `message` and remaining fields separately.
#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: String,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            let _ = write!(self.fields, "{}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            let _ = write!(self.fields, "{}={value}", field.name());
        }
    }
}

/// `tracing` layer that mirrors every event into the in-memory ring buffer.
pub struct MemoryLayer;

impl<S: Subscriber> Layer<S> for MemoryLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        LOG_STORE.push(
            level_str(meta.level()),
            meta.target(),
            visitor.message,
            visitor.fields,
        );
    }
}

/// Construct the capture layer for registration in the tracing subscriber.
pub fn layer() -> MemoryLayer {
    MemoryLayer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_snapshot_and_level_filter() {
        clear();
        LOG_STORE.push("info", "t", "hello".into(), String::new());
        LOG_STORE.push("warn", "t", "careful".into(), "code=5".into());
        LOG_STORE.push("error", "t", "boom".into(), String::new());

        let all = snapshot(100, "trace");
        assert_eq!(all.len(), 3);
        assert!(all[0].seq < all[1].seq && all[1].seq < all[2].seq);

        let warn_plus = snapshot(100, "warn");
        assert_eq!(warn_plus.len(), 2);
        assert_eq!(warn_plus[0].level, "warn");
        assert_eq!(warn_plus[1].level, "error");
    }

    #[test]
    fn limit_keeps_newest() {
        clear();
        for i in 0..10 {
            LOG_STORE.push("info", "t", format!("m{i}"), String::new());
        }
        let last3 = snapshot(3, "trace");
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].message, "m7");
        assert_eq!(last3[2].message, "m9");
    }

    #[test]
    fn clear_empties_buffer() {
        clear();
        LOG_STORE.push("info", "t", "x".into(), String::new());
        assert!(clear() >= 1);
        assert_eq!(snapshot(10, "trace").len(), 0);
    }
}
