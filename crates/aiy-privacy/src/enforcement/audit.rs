//! Privacy audit logging
//!
//! Provides metadata-only logging of redaction activity. This module
//! ensures that audit logs contain ONLY:
//! - Timestamps
//! - Counts (number of redactions)
//! - Hashes (for correlation without revealing content)
//! - Categories/types
//!
//! Audit logs must NEVER contain:
//! - Original identifiers
//! - Opaque ID mappings
//! - File paths or code content

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::redactor::RedactionStats;

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventKind {
    /// Redaction session started
    SessionStart,
    /// Redaction performed
    Redaction,
    /// Unredaction performed (session-local)
    Unredaction,
    /// Privacy violation blocked
    ViolationBlocked,
    /// Privacy violation warned
    ViolationWarned,
    /// Session cleared
    SessionClear,
    /// Session ended
    SessionEnd,
}

impl AuditEventKind {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventKind::SessionStart => "session_start",
            AuditEventKind::Redaction => "redaction",
            AuditEventKind::Unredaction => "unredaction",
            AuditEventKind::ViolationBlocked => "violation_blocked",
            AuditEventKind::ViolationWarned => "violation_warned",
            AuditEventKind::SessionClear => "session_clear",
            AuditEventKind::SessionEnd => "session_end",
        }
    }
}

/// A single audit event (metadata only)
///
/// # Security
///
/// This struct intentionally stores only metadata, never actual content:
/// - Timestamps (when)
/// - Counts (how many)
/// - Hashes (correlation keys, not content)
/// - Event types (what kind of action)
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Unix timestamp (seconds)
    pub timestamp: u64,
    /// Event type
    pub kind: AuditEventKind,
    /// Count of items (e.g., number of redactions)
    pub count: usize,
    /// Breakdown by category (e.g., 3 files, 2 functions)
    pub by_category: HashMap<String, u32>,
    /// Session hash (for correlation, not the session ID)
    pub session_hash: String,
}

impl AuditEvent {
    /// Create a new audit event
    fn new(kind: AuditEventKind, session_hash: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            timestamp,
            kind,
            count: 0,
            by_category: HashMap::new(),
            session_hash: session_hash.to_string(),
        }
    }

    /// Format as a log line (safe, metadata only)
    pub fn to_log_line(&self) -> String {
        let mut parts = vec![
            format!("ts={}", self.timestamp),
            format!("event={}", self.kind.as_str()),
            format!("session={}", &self.session_hash[..8]), // Truncate for logs
        ];

        if self.count > 0 {
            parts.push(format!("count={}", self.count));
        }

        if !self.by_category.is_empty() {
            let categories: Vec<String> = self
                .by_category
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect();
            parts.push(format!("breakdown={}", categories.join(",")));
        }

        parts.join(" ")
    }
}

/// Redaction audit logger
///
/// Records metadata about redaction activity without storing any
/// sensitive identifiers. Logs are suitable for compliance and
/// debugging without compromising privacy.
#[derive(Debug)]
pub struct RedactionAudit {
    /// Events recorded this session
    events: Vec<AuditEvent>,
    /// Session hash (derived from session ID, not the ID itself)
    session_hash: String,
    /// Running totals
    total_redactions: usize,
    total_unredactions: usize,
    violations_blocked: usize,
    violations_warned: usize,
}

impl RedactionAudit {
    /// Create a new audit logger
    ///
    /// Takes a session hash (not the session ID itself) for correlation.
    pub fn new(session_hash: &str) -> Self {
        let mut audit = Self {
            events: Vec::new(),
            session_hash: session_hash.to_string(),
            total_redactions: 0,
            total_unredactions: 0,
            violations_blocked: 0,
            violations_warned: 0,
        };

        // Log session start
        audit.events.push(AuditEvent::new(
            AuditEventKind::SessionStart,
            &audit.session_hash,
        ));

        audit
    }

    /// Create from a UUID (hashes it for privacy)
    pub fn from_session_id(session_id: &uuid::Uuid) -> Self {
        // Hash the session ID so we don't log it directly
        let hash = format!("{:x}", md5_hash(session_id.as_bytes()));
        Self::new(&hash)
    }

    /// Log a redaction event
    ///
    /// Records the count and breakdown by category, never the actual identifiers.
    pub fn log_redaction(&mut self, stats: &RedactionStats) {
        let mut event = AuditEvent::new(AuditEventKind::Redaction, &self.session_hash);
        event.count = stats.total;

        for (kind, count) in &stats.by_kind {
            event.by_category.insert(kind.prefix().to_lowercase(), *count);
        }

        self.total_redactions += stats.total;
        self.events.push(event);
    }

    /// Log an unredaction event
    pub fn log_unredaction(&mut self, count: usize) {
        let mut event = AuditEvent::new(AuditEventKind::Unredaction, &self.session_hash);
        event.count = count;
        self.total_unredactions += count;
        self.events.push(event);
    }

    /// Log a blocked violation
    pub fn log_violation_blocked(&mut self, violation_count: usize, categories: &[&str]) {
        let mut event = AuditEvent::new(AuditEventKind::ViolationBlocked, &self.session_hash);
        event.count = violation_count;

        for cat in categories {
            *event.by_category.entry(cat.to_string()).or_insert(0) += 1;
        }

        self.violations_blocked += violation_count;
        self.events.push(event);
    }

    /// Log a warned violation
    pub fn log_violation_warned(&mut self, violation_count: usize, categories: &[&str]) {
        let mut event = AuditEvent::new(AuditEventKind::ViolationWarned, &self.session_hash);
        event.count = violation_count;

        for cat in categories {
            *event.by_category.entry(cat.to_string()).or_insert(0) += 1;
        }

        self.violations_warned += violation_count;
        self.events.push(event);
    }

    /// Log session clear
    pub fn log_session_clear(&mut self) {
        self.events.push(AuditEvent::new(
            AuditEventKind::SessionClear,
            &self.session_hash,
        ));
    }

    /// Log session end
    pub fn log_session_end(&mut self) {
        self.events.push(AuditEvent::new(
            AuditEventKind::SessionEnd,
            &self.session_hash,
        ));
    }

    /// Get all events
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get summary statistics (safe to log)
    pub fn summary(&self) -> AuditSummary {
        AuditSummary {
            session_hash: self.session_hash[..8].to_string(),
            total_redactions: self.total_redactions,
            total_unredactions: self.total_unredactions,
            violations_blocked: self.violations_blocked,
            violations_warned: self.violations_warned,
            event_count: self.events.len(),
        }
    }

    /// Format as log lines (all events)
    pub fn to_log_lines(&self) -> Vec<String> {
        self.events.iter().map(|e| e.to_log_line()).collect()
    }
}

/// Audit summary (safe to log/serialize)
#[derive(Debug, Clone)]
pub struct AuditSummary {
    /// Truncated session hash
    pub session_hash: String,
    /// Total redactions performed
    pub total_redactions: usize,
    /// Total unredactions performed
    pub total_unredactions: usize,
    /// Violations blocked
    pub violations_blocked: usize,
    /// Violations warned
    pub violations_warned: usize,
    /// Number of audit events
    pub event_count: usize,
}

impl AuditSummary {
    /// Format as a human-readable summary
    pub fn to_string(&self) -> String {
        format!(
            "Privacy audit [{}]: {} redactions, {} unredactions, {} blocked, {} warned ({} events)",
            self.session_hash,
            self.total_redactions,
            self.total_unredactions,
            self.violations_blocked,
            self.violations_warned,
            self.event_count
        )
    }
}

/// Simple MD5 hash (for audit correlation, not security)
fn md5_hash(data: &[u8]) -> u128 {
    // Simple hash for correlation purposes only
    // This is NOT for security - just for audit log correlation
    let mut hash: u128 = 0;
    for (i, &byte) in data.iter().enumerate() {
        hash = hash.wrapping_add((byte as u128) << ((i % 16) * 8));
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::redactor::RedactionKind;

    #[test]
    fn test_audit_new() {
        let audit = RedactionAudit::new("test_session_hash_1234");
        assert_eq!(audit.event_count(), 1); // session_start event
        assert_eq!(audit.events()[0].kind, AuditEventKind::SessionStart);
    }

    #[test]
    fn test_log_redaction() {
        let mut audit = RedactionAudit::new("test_session");
        let mut by_kind = HashMap::new();
        by_kind.insert(RedactionKind::File, 3);
        by_kind.insert(RedactionKind::Function, 2);

        let stats = RedactionStats {
            total: 5,
            by_kind,
        };

        audit.log_redaction(&stats);

        let summary = audit.summary();
        assert_eq!(summary.total_redactions, 5);
        assert_eq!(audit.event_count(), 2); // start + redaction
    }

    #[test]
    fn test_log_violation_blocked() {
        let mut audit = RedactionAudit::new("test_session");
        audit.log_violation_blocked(3, &["FilePath", "SourceCode"]);

        let summary = audit.summary();
        assert_eq!(summary.violations_blocked, 3);
    }

    #[test]
    fn test_log_violation_warned() {
        let mut audit = RedactionAudit::new("test_session");
        audit.log_violation_warned(2, &["Secret"]);

        let summary = audit.summary();
        assert_eq!(summary.violations_warned, 2);
    }

    #[test]
    fn test_audit_event_to_log_line() {
        let mut event = AuditEvent::new(AuditEventKind::Redaction, "abcdef1234567890");
        event.count = 5;
        event.by_category.insert("file".to_string(), 3);
        event.by_category.insert("func".to_string(), 2);

        let line = event.to_log_line();
        assert!(line.contains("event=redaction"));
        assert!(line.contains("count=5"));
        assert!(line.contains("session=abcdef12"));
    }

    #[test]
    fn test_audit_summary() {
        let mut audit = RedactionAudit::new("test_session_hash");
        audit.log_violation_blocked(2, &["FilePath"]);
        audit.log_violation_warned(1, &["Secret"]);

        let summary = audit.summary();
        let summary_str = summary.to_string();

        assert!(summary_str.contains("2 blocked"));
        assert!(summary_str.contains("1 warned"));
    }

    #[test]
    fn test_to_log_lines() {
        let mut audit = RedactionAudit::new("test_session");
        audit.log_session_clear();
        audit.log_session_end();

        let lines = audit.to_log_lines();
        assert_eq!(lines.len(), 3); // start, clear, end
    }

    #[test]
    fn test_from_session_id() {
        let session_id = uuid::Uuid::new_v4();
        let audit = RedactionAudit::from_session_id(&session_id);

        // Should have session_start event
        assert_eq!(audit.event_count(), 1);
        // Session hash should not be the UUID itself
        assert!(!audit.session_hash.contains(&session_id.to_string()));
    }

    #[test]
    fn test_audit_no_sensitive_data_in_logs() {
        let mut audit = RedactionAudit::new("test_session_hash_abcdef");

        // Log some activity
        let mut by_kind = HashMap::new();
        by_kind.insert(RedactionKind::File, 47);
        by_kind.insert(RedactionKind::Function, 12);

        let stats = RedactionStats {
            total: 59,
            by_kind,
        };
        audit.log_redaction(&stats);

        // Get all log lines
        let lines = audit.to_log_lines();

        // Verify logs contain only metadata
        for line in &lines {
            // Should not contain any actual paths
            assert!(!line.contains("/home"));
            assert!(!line.contains("/usr"));
            assert!(!line.contains("src/"));
            // Should not contain code
            assert!(!line.contains("fn "));
            assert!(!line.contains("pub "));
            // Should contain metadata
            assert!(line.contains("ts=") || line.contains("event="));
        }

        // Check summary is also safe
        let summary = audit.summary();
        let summary_str = summary.to_string();
        assert!(summary_str.contains("59 redactions"));
        assert!(!summary_str.contains("/home"));
    }
}
