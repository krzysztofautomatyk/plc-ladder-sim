//! =============================================================================
//! Audit Trail — military/medical grade cryptographic hash chaining.
//! Each entry hashes: prev_hash || timestamp || actor || action || detail.
//! Exportable report for regulatory review (IEC 62304 / 21 CFR Part 11 style).
//! =============================================================================

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub detail: String,
    pub program_hash: String,
    pub prev_hash: String,
    pub entry_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub generated_at: DateTime<Utc>,
    pub entry_count: usize,
    pub chain_valid: bool,
    pub head_hash: String,
    pub entries: Vec<AuditEntry>,
}

pub struct AuditTrail {
    entries: RwLock<Vec<AuditEntry>>,
    log_path: RwLock<Option<PathBuf>>,
}

impl AuditTrail {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: RwLock::new(Vec::new()),
            log_path: RwLock::new(None),
        })
    }

    pub fn set_log_path(&self, path: PathBuf) {
        *self.log_path.write() = Some(path);
    }

    /// Load previously persisted entries from the append-only log so the
    /// cryptographic hash chain **continues across application restarts**.
    ///
    /// Returns `(loaded_count, chain_intact)`. `chain_intact` is `false` if any
    /// line failed to parse or the recomputed hash chain does not validate —
    /// i.e. the on-disk trail was tampered with or truncated.
    pub fn load_persisted(&self) -> (usize, bool) {
        let path = self.log_path.read().clone();
        let Some(path) = path else {
            return (0, true);
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            // No file yet (first run) is not an integrity failure.
            Err(_) => return (0, true),
        };

        let mut loaded: Vec<AuditEntry> = Vec::new();
        let mut parse_ok = true;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditEntry>(line) {
                Ok(e) => loaded.push(e),
                Err(e) => {
                    parse_ok = false;
                    warn!(error = %e, "audit log: unparseable line — chain integrity suspect");
                }
            }
        }

        *self.entries.write() = loaded;
        let count = self.entries.read().len();
        let valid = parse_ok && self.verify_chain();
        if count > 0 {
            info!(count, valid, "audit trail restored from disk");
        }
        (count, valid)
    }

    /// Append immutable audit record with hash chain.
    pub fn record(
        &self,
        actor: impl Into<String>,
        action: impl Into<String>,
        detail: impl Into<String>,
        program_hash: impl Into<String>,
    ) -> AuditEntry {
        let actor = actor.into();
        let action = action.into();
        let detail = detail.into();
        let program_hash = program_hash.into();
        let timestamp = Utc::now();

        let mut entries = self.entries.write();
        let sequence = entries.len() as u64 + 1;
        let prev_hash = entries
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string());

        let entry_hash = compute_hash(
            &prev_hash,
            &timestamp,
            &actor,
            &action,
            &detail,
            &program_hash,
            sequence,
        );

        let entry = AuditEntry {
            id: Uuid::new_v4().to_string(),
            sequence,
            timestamp,
            actor: actor.clone(),
            action: action.clone(),
            detail: detail.clone(),
            program_hash,
            prev_hash,
            entry_hash,
        };

        // Append-only file mirror
        if let Some(path) = self.log_path.read().as_ref() {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
                if let Ok(line) = serde_json::to_string(&entry) {
                    let _ = writeln!(f, "{line}");
                }
            }
        }

        info!(
            sequence = entry.sequence,
            action = %entry.action,
            hash = %entry.entry_hash,
            "audit"
        );

        entries.push(entry.clone());
        entry
    }

    pub fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().clone()
    }

    pub fn verify_chain(&self) -> bool {
        let entries = self.entries.read();
        let mut expected_prev = GENESIS_HASH.to_string();
        for (i, e) in entries.iter().enumerate() {
            if e.prev_hash != expected_prev {
                warn!(sequence = e.sequence, "audit chain break (prev_hash)");
                return false;
            }
            let recomputed = compute_hash(
                &e.prev_hash,
                &e.timestamp,
                &e.actor,
                &e.action,
                &e.detail,
                &e.program_hash,
                e.sequence,
            );
            if recomputed != e.entry_hash {
                warn!(sequence = e.sequence, "audit chain break (entry_hash)");
                return false;
            }
            if e.sequence != (i as u64 + 1) {
                return false;
            }
            expected_prev = e.entry_hash.clone();
        }
        true
    }

    pub fn report(&self) -> AuditReport {
        let entries = self.entries();
        let chain_valid = self.verify_chain();
        let head_hash = entries
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string());
        AuditReport {
            generated_at: Utc::now(),
            entry_count: entries.len(),
            chain_valid,
            head_hash,
            entries,
        }
    }
}

fn compute_hash(
    prev_hash: &str,
    timestamp: &DateTime<Utc>,
    actor: &str,
    action: &str,
    detail: &str,
    program_hash: &str,
    sequence: u64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.to_rfc3339().as_bytes());
    hasher.update(b"|");
    hasher.update(actor.as_bytes());
    hasher.update(b"|");
    hasher.update(action.as_bytes());
    hasher.update(b"|");
    hasher.update(detail.as_bytes());
    hasher.update(b"|");
    hasher.update(program_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(sequence.to_string().as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_valid_after_records() {
        let trail = AuditTrail::new();
        trail.record("user", "START", "sim", "abc");
        trail.record("user", "STOP", "sim", "abc");
        assert!(trail.verify_chain());
        assert_eq!(trail.entries().len(), 2);
    }

    fn temp_log() -> PathBuf {
        std::env::temp_dir().join(format!("plc_audit_test_{}.jsonl", Uuid::new_v4()))
    }

    #[test]
    fn persisted_chain_continues_across_restart() {
        let path = temp_log();

        // Session 1: two entries mirrored to disk.
        let a = AuditTrail::new();
        a.set_log_path(path.clone());
        a.record("operator", "START_SIMULATION", "cycle_ms=20", "hash1");
        a.record("operator", "STOP_SIMULATION", "scan_count=5", "hash1");

        // Session 2: fresh trail restores from disk and keeps chaining.
        let b = AuditTrail::new();
        b.set_log_path(path.clone());
        let (count, intact) = b.load_persisted();
        assert_eq!(count, 2, "both entries restored");
        assert!(intact, "restored chain must validate");

        // New record continues the same chain (prev_hash = restored head).
        let e3 = b.record("operator", "UPDATE_PROGRAM", "hash=hash2", "hash2");
        assert_eq!(e3.sequence, 3, "sequence continues across restart");
        assert!(b.verify_chain(), "chain still valid after continuation");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tampered_persisted_log_is_detected() {
        let path = temp_log();

        let a = AuditTrail::new();
        a.set_log_path(path.clone());
        a.record("operator", "START_SIMULATION", "cycle_ms=20", "hash1");
        a.record("operator", "SET_DISCRETE", "I0=true", "hash1");

        // Tamper: rewrite the log with a modified detail field on the first entry.
        let original = std::fs::read_to_string(&path).unwrap();
        let tampered = original.replacen("cycle_ms=20", "cycle_ms=99", 1);
        assert_ne!(original, tampered, "tamper must actually change content");
        std::fs::write(&path, tampered).unwrap();

        let b = AuditTrail::new();
        b.set_log_path(path.clone());
        let (count, intact) = b.load_persisted();
        assert_eq!(count, 2);
        assert!(!intact, "tampered on-disk trail must fail integrity check");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_log_is_not_an_integrity_failure() {
        let a = AuditTrail::new();
        a.set_log_path(temp_log()); // path that does not exist yet
        let (count, intact) = a.load_persisted();
        assert_eq!(count, 0);
        assert!(intact);
    }
}
