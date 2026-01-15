//! # aiy-privacy
//!
//! Privacy enforcement crate for All-in-Yum's privacy mode.
//!
//! This crate provides comprehensive privacy protection to ensure that
//! sensitive code, file paths, and identifiers are never sent to cloud
//! services when privacy mode is enabled.
//!
//! ## Features
//!
//! - **Privacy Policy**: 25+ patterns to detect violations (paths, code, secrets)
//! - **Privacy Guard**: Runtime enforcement with Warn/Block/Panic modes
//! - **Redactor**: In-memory identifier redaction (FILE_001, FUNC_002, etc.)
//! - **Audit**: Metadata-only logging (counts, timestamps, never identifiers)
//!
//! ## Security Model
//!
//! The privacy enforcement follows a defense-in-depth approach:
//!
//! 1. **Detection Layer**: [`enforcement::PrivacyPolicy`] scans content for
//!    patterns that indicate sensitive information.
//!
//! 2. **Enforcement Layer**: [`enforcement::PrivacyGuard`] blocks or warns
//!    when violations are detected before content reaches cloud services.
//!
//! 3. **Redaction Layer**: [`enforcement::Redactor`] provides opaque
//!    identifier replacement so users can still describe code issues
//!    without revealing actual paths or names.
//!
//! 4. **Audit Layer**: [`enforcement::RedactionAudit`] provides compliance
//!    logging with metadata only - never actual identifiers.
//!
//! ## Critical Security Requirements
//!
//! - **RedactionMap is NOT Serializable**: The mapping between original
//!   identifiers and opaque IDs must never be persisted to disk or sent
//!   over the network. This is enforced by intentionally not implementing
//!   `Serialize` or `Deserialize` on [`enforcement::RedactionMap`].
//!
//! - **Session-Scoped Redaction**: Each session gets a fresh redaction map.
//!   Mappings are destroyed when the session ends.
//!
//! - **Audit Logs Contain Metadata Only**: Audit logs record counts,
//!   timestamps, and categories - never actual file paths, code, or
//!   identifier names.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use aiy_privacy::enforcement::{PrivacyGuard, GuardMode, Redactor, RedactionAudit};
//!
//! // Initialize components
//! let guard = PrivacyGuard::with_mode(GuardMode::Block);
//! let mut redactor = Redactor::new();
//! let mut audit = RedactionAudit::from_session_id(redactor.map().session_id());
//!
//! // User describes an issue with code
//! let user_input = "I have an error in /home/user/myproject/src/auth.rs";
//!
//! // Check if it contains violations
//! if !guard.is_safe(user_input) {
//!     // Redact sensitive paths
//!     let paths = ["/home/user/myproject/src/auth.rs"];
//!     let redacted = redactor.redact_paths(user_input, &paths);
//!
//!     // Log redaction (metadata only)
//!     audit.log_redaction(&redactor.stats());
//!
//!     // Now safe to send
//!     assert!(guard.is_safe(&redacted));
//!     println!("Redacted: {}", redacted);
//!     // Output: "I have an error in FILE_001"
//! }
//!
//! // When AI responds, unredact for display
//! let ai_response = "The error in FILE_001 suggests a type mismatch";
//! let display = redactor.unredact_content(ai_response);
//! // Output: "The error in /home/user/myproject/src/auth.rs suggests..."
//! ```
//!
//! ## Module Organization
//!
//! - [`enforcement`] - Core privacy enforcement modules
//!   - [`enforcement::policy`] - Violation detection patterns
//!   - [`enforcement::guard`] - Runtime enforcement
//!   - [`enforcement::redactor`] - Identifier redaction
//!   - [`enforcement::audit`] - Metadata-only audit logging
//!
//! - [`rag`] - Local RAG (Retrieval-Augmented Generation) system
//!   - [`rag::types`] - Core types (CodeChunk, ChunkId, etc.)
//!   - [`rag::chunk`] - Chunking strategies
//!   - [`rag::store`] - In-memory vector store
//!   - [`rag::embedder`] - Local embedding generation
//!   - [`rag::indexer`] - Directory indexing
//!   - [`rag::query`] - Query processing
//!
//! - [`verification`] - Verification engine for privacy mode
//!   - [`verification::types`] - Result and failure types
//!   - [`verification::stages`] - Verification stages (fmt, clippy, test)
//!   - [`verification::repair`] - Automatic repair generation
//!   - [`verification::engine`] - Main verification engine

pub mod enforcement;
pub mod rag;
pub mod verification;

// Re-export commonly used types at crate root
pub use enforcement::{
    GuardError, GuardMode, PrivacyGuard, PrivacyPolicy, RedactionAudit, RedactionMap, Redactor,
    Violation, ViolationCategory,
};

// Re-export RAG types
pub use rag::{
    ChunkId, ChunkType, ChunkingStrategy, CodeChunk, CodeIndexer, Embedder, InMemoryVectorStore,
    LocalEmbedder, QueryProcessor, QueryResult, RagError, RagSystem, RagSystemConfig, RankedChunk,
};

// Re-export Verification types
pub use verification::{
    BackoffConfig, ClippyStage, CodeLocation, DiagnosticSeverity, EngineConfig, FailureType,
    FmtStage, Repair, RepairConfig, RepairGenerator, RepairSummary, StageConfig, StageFailure,
    StageResult, TestStage, VerificationEngine, VerificationError, VerificationResult,
    VerificationStage, VerificationState,
};
