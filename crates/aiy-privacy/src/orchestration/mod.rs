//! Subagent orchestration layer for privacy mode
//!
//! This module provides the orchestration infrastructure for privacy mode:
//!
//! - **Cloud Communicator**: Sends redacted context to cloud models for planning
//! - **Local Executor**: Executes plans locally with full code access
//! - **Session Management**: Tracks in-memory state (never persists sensitive data)
//! - **Plan Parsing**: Parses execution plans from cloud responses
//!
//! # Security Model
//!
//! The orchestration layer maintains strict separation:
//!
//! 1. **Cloud Side**: Only receives redacted content (FILE_001, FUNC_002, etc.)
//!    - No actual file paths
//!    - No source code
//!    - No identifier names
//!
//! 2. **Local Side**: Has full access to code for implementation
//!    - Redaction map stays in-memory only
//!    - Verification runs locally
//!    - All code changes are local
//!
//! # Usage
//!
//! ```rust,ignore
//! use aiy_privacy::orchestration::{PrivacyOrchestrator, OrchestratorConfig};
//!
//! // Create orchestrator
//! let config = OrchestratorConfig::from_privacy_config(&privacy_config, "/path/to/repo")?;
//! let mut orchestrator = PrivacyOrchestrator::new(config);
//!
//! // Initialize (indexes codebase)
//! orchestrator.init().await?;
//!
//! // Execute request
//! let stats = orchestrator.execute("Add logging to the auth module").await?;
//!
//! // Check results
//! println!("Tasks executed: {}", stats.tasks_executed);
//! ```

pub mod cloud;
pub mod error;
pub mod local;
pub mod orchestrator;
pub mod plan;
pub mod session;
pub mod types;

// Re-export main types
pub use cloud::{CloudCommunicator, CloudRequestBuilder};
pub use error::{OrchestrationError, OrchestrationResult};
pub use local::{LocalExecutor, LocalExecutorConfig};
pub use orchestrator::{OrchestratorConfig, PrivacyOrchestrator};
pub use plan::PlanParser;
pub use session::{OrchestrationSession, SessionState};
pub use types::{ExecutionPlan, PlanTask, SessionStats, TaskResult, TaskType};
