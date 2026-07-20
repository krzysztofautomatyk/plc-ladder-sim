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

const GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

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

    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.report())
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
}
