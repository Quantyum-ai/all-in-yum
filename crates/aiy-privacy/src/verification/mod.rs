//! Verification engine for Privacy Mode 2.5.3b.
//!
//! This module provides a complete verification pipeline that runs cargo commands
//! (fmt, clippy, test) and attempts automatic repairs using a local Ollama model.
//!
//! ## Features
//!
//! - **Multi-stage verification**: Runs fmt -> clippy -> test in sequence
//! - **Automatic repair**: Uses Ollama to generate fixes for failures
//! - **Rate limiting**: Enforces per-stage and global repair limits (2/2/1/8 default)
//! - **Privacy enforcement**: All code stays local, no cloud transmission
//! - **Structured output**: Parses cargo JSON for detailed diagnostics
//!
//! ## Security
//!
//! The verification engine is designed for privacy mode:
//! - All code processing happens locally via Ollama
//! - Repair outputs are scanned by `PrivacyGuard` before application
//! - No source code is ever sent to cloud services
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use aiy_privacy::verification::{VerificationEngine, EngineConfig};
//! use aiy_adapter_ollama::OllamaAdapter;
//! use std::path::Path;
//!
//! // Create engine configuration
//! let config = EngineConfig::new(Path::new("/path/to/project"));
//!
//! // Create engine with Ollama adapter
//! let mut engine = VerificationEngine::new(config)
//!     .with_ollama_adapter(adapter);
//!
//! // Run verification
//! let result = engine.run().await?;
//!
//! if result.success {
//!     println!("Verification passed with {} repairs", result.total_repairs);
//! } else {
//!     println!("Verification failed: {}", result.summary);
//! }
//! ```
//!
//! ## Repair Limits
//!
//! The engine enforces configurable repair limits from `VerificationConfig`:
//!
//! | Stage   | Default Limit | Description |
//! |---------|---------------|-------------|
//! | fmt     | 2             | Max formatting repairs |
//! | clippy  | 2             | Max lint repairs |
//! | test    | 1             | Max test repairs |
//! | global  | 8             | Max repairs across all stages |
//!
//! ## Module Organization
//!
//! - [`types`] - Result and failure type definitions
//! - [`error`] - Verification error types
//! - [`state`] - Verification state machine
//! - [`stages`] - Individual verification stages (fmt, clippy, test)
//! - [`repair`] - Repair generation using Ollama
//! - [`engine`] - Main verification engine

pub mod engine;
pub mod error;
pub mod repair;
pub mod stages;
pub mod state;
pub mod subagent;
pub mod types;

// Re-export commonly used types
pub use engine::{BackoffConfig, EngineConfig, VerificationEngine};
pub use error::{aggregate_failures, VerificationError, VerificationResult as VerificationResultType};
pub use repair::{Repair, RepairConfig, RepairGenerator};
pub use stages::{
    default_stage_order, stage_by_name, BuildStage, ClippyStage, FmtStage, StageConfig, TestStage,
    VerificationStage,
};
pub use subagent::SubagentCoordinator;
pub use state::VerificationState;
pub use types::{
    CodeLocation, DiagnosticSeverity, FailureType, RelatedInfo, RepairSummary, StageFailure,
    StageResult, VerificationResult,
};
