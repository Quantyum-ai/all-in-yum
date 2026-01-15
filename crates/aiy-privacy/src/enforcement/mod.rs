//! Privacy enforcement modules
//!
//! This module provides the core privacy enforcement capabilities:
//!
//! - [`policy`] - Privacy violation patterns and detection
//! - [`guard`] - Runtime enforcement (Warn/Block/Panic modes)
//! - [`redactor`] - Identifier redaction (in-memory only)
//! - [`audit`] - Metadata-only audit logging
//!
//! # Security Model
//!
//! The privacy enforcement follows these principles:
//!
//! 1. **Detection**: The [`PrivacyPolicy`] contains 25+ patterns to detect
//!    file paths, source code, diffs, secrets, and real identifiers.
//!
//! 2. **Enforcement**: The [`PrivacyGuard`] intercepts outgoing payloads
//!    and enforces the policy in one of three modes:
//!    - `Warn`: Log violations but allow (development)
//!    - `Block`: Reject payloads with violations (production)
//!    - `Panic`: Crash on violations (security audit)
//!
//! 3. **Redaction**: The [`Redactor`] replaces sensitive identifiers with
//!    opaque placeholders (FILE_001, FUNC_002, etc.). The mapping is:
//!    - In-memory only (never serialized)
//!    - Session-scoped (destroyed when session ends)
//!    - Reversible within session only
//!
//! 4. **Audit**: The [`RedactionAudit`] logs metadata only:
//!    - Timestamps, counts, categories
//!    - Never actual identifiers or mappings
//!
//! # Example
//!
//! ```rust,ignore
//! use aiy_privacy::enforcement::{PrivacyGuard, GuardMode, Redactor};
//!
//! // Set up privacy guard
//! let guard = PrivacyGuard::with_mode(GuardMode::Block);
//!
//! // Set up redactor
//! let mut redactor = Redactor::new();
//!
//! // Process content
//! let content = "Error in /home/user/src/main.rs";
//! let paths = ["/home/user/src/main.rs"];
//! let redacted = redactor.redact_paths(content, &paths);
//!
//! // Now safe to send (FILE_001 instead of path)
//! assert!(guard.check(&redacted).is_ok());
//! ```

pub mod audit;
pub mod guard;
pub mod policy;
pub mod redactor;

// Re-export key types for convenience
pub use audit::{AuditEvent, AuditEventKind, AuditSummary, RedactionAudit};
pub use guard::{AsyncGuard, GuardError, GuardMode, PrivacyGuard};
pub use policy::{PrivacyPolicy, Violation, ViolationCategory};
pub use redactor::{RedactionKind, RedactionMap, RedactionStats, Redactor};
