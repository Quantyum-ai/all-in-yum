//! Error types for the verification engine.
//!
//! This module defines all error types that can occur during verification,
//! including stage execution errors, repair failures, and limit violations.

use super::types::{FailureType, StageFailure};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during verification.
#[derive(Debug, Error)]
pub enum VerificationError {
    /// Stage execution failed with cargo error
    #[error("Stage '{stage}' execution failed: {message}")]
    StageExecution {
        /// Name of the stage that failed
        stage: String,
        /// Error message from the stage
        message: String,
        /// Exit code if available
        exit_code: Option<i32>,
    },

    /// Repair generation failed
    #[error("Failed to generate repair for {failure_type}: {message}")]
    RepairGeneration {
        /// Type of failure being repaired
        failure_type: FailureType,
        /// Error message
        message: String,
    },

    /// Repair application failed
    #[error("Failed to apply repair to {file}: {message}")]
    RepairApplication {
        /// File being repaired
        file: PathBuf,
        /// Error message
        message: String,
    },

    /// Stage-specific repair limit exceeded
    #[error("Repair limit exceeded for stage '{stage}': {attempts}/{limit} attempts")]
    StageLimitExceeded {
        /// Name of the stage
        stage: String,
        /// Number of attempts made
        attempts: usize,
        /// Maximum allowed attempts
        limit: usize,
    },

    /// Global repair limit exceeded
    #[error("Global repair limit exceeded: {attempts}/{limit} attempts")]
    GlobalLimitExceeded {
        /// Number of attempts made
        attempts: usize,
        /// Maximum allowed attempts
        limit: usize,
    },

    /// Verification pipeline was cancelled
    #[error("Verification cancelled: {reason}")]
    Cancelled {
        /// Reason for cancellation
        reason: String,
    },

    /// No stages configured
    #[error("No verification stages configured")]
    NoStagesConfigured,

    /// Working directory not found or invalid
    #[error("Invalid working directory: {path}")]
    InvalidWorkingDirectory {
        /// The invalid path
        path: PathBuf,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// IO error during verification
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error (for cargo JSON output)
    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Ollama adapter error during repair generation
    #[error("Ollama error: {0}")]
    Ollama(String),

    /// Privacy guard blocked the repair output
    #[error("Privacy guard blocked repair output: {reason}")]
    PrivacyViolation {
        /// Reason for blocking
        reason: String,
    },

    /// Timeout during stage execution
    #[error("Stage '{stage}' timed out after {timeout_secs}s")]
    Timeout {
        /// Name of the stage
        stage: String,
        /// Timeout in seconds
        timeout_secs: u64,
    },

    /// Compile error detected (separate from clippy warnings)
    #[error("Compilation error: {0}")]
    CompileError(String),

    /// Verification state error (invalid state transition)
    #[error("Invalid state transition: {0}")]
    InvalidState(String),
}

impl VerificationError {
    /// Create a stage execution error
    pub fn stage_execution(stage: &str, message: impl Into<String>, exit_code: Option<i32>) -> Self {
        Self::StageExecution {
            stage: stage.to_string(),
            message: message.into(),
            exit_code,
        }
    }

    /// Create a repair generation error
    pub fn repair_generation(failure_type: FailureType, message: impl Into<String>) -> Self {
        Self::RepairGeneration {
            failure_type,
            message: message.into(),
        }
    }

    /// Create a repair application error
    pub fn repair_application(file: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::RepairApplication {
            file: file.into(),
            message: message.into(),
        }
    }

    /// Create a stage limit exceeded error
    pub fn stage_limit_exceeded(stage: &str, attempts: usize, limit: usize) -> Self {
        Self::StageLimitExceeded {
            stage: stage.to_string(),
            attempts,
            limit,
        }
    }

    /// Create a global limit exceeded error
    pub fn global_limit_exceeded(attempts: usize, limit: usize) -> Self {
        Self::GlobalLimitExceeded { attempts, limit }
    }

    /// Create a cancellation error
    pub fn cancelled(reason: impl Into<String>) -> Self {
        Self::Cancelled {
            reason: reason.into(),
        }
    }

    /// Create an invalid working directory error
    pub fn invalid_working_directory(path: impl Into<PathBuf>) -> Self {
        Self::InvalidWorkingDirectory { path: path.into() }
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    /// Create an Ollama error
    pub fn ollama(message: impl Into<String>) -> Self {
        Self::Ollama(message.into())
    }

    /// Create a privacy violation error
    pub fn privacy_violation(reason: impl Into<String>) -> Self {
        Self::PrivacyViolation {
            reason: reason.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(stage: &str, timeout_secs: u64) -> Self {
        Self::Timeout {
            stage: stage.to_string(),
            timeout_secs,
        }
    }

    /// Create a compile error
    pub fn compile_error(message: impl Into<String>) -> Self {
        Self::CompileError(message.into())
    }

    /// Create an invalid state error
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::InvalidState(message.into())
    }

    /// Check if this error is recoverable (can continue verification)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::StageExecution { .. }
                | Self::RepairGeneration { .. }
                | Self::RepairApplication { .. }
        )
    }

    /// Check if this error indicates a limit was exceeded
    pub fn is_limit_exceeded(&self) -> bool {
        matches!(
            self,
            Self::StageLimitExceeded { .. } | Self::GlobalLimitExceeded { .. }
        )
    }

    /// Check if this error is a timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }

    /// Get the stage name if this error is stage-related
    pub fn stage_name(&self) -> Option<&str> {
        match self {
            Self::StageExecution { stage, .. } => Some(stage),
            Self::StageLimitExceeded { stage, .. } => Some(stage),
            Self::Timeout { stage, .. } => Some(stage),
            _ => None,
        }
    }
}

/// Result type for verification operations
pub type VerificationResult<T> = Result<T, VerificationError>;

/// Aggregate multiple failures into a single error message
pub fn aggregate_failures(failures: &[StageFailure]) -> String {
    if failures.is_empty() {
        return String::new();
    }

    let mut messages = Vec::with_capacity(failures.len());
    for (i, failure) in failures.iter().enumerate().take(5) {
        let location = failure
            .location
            .as_ref()
            .map(|l| l.display())
            .unwrap_or_else(|| "unknown".to_string());
        messages.push(format!(
            "{}. [{}] {}: {}",
            i + 1,
            failure.failure_type,
            location,
            failure.message
        ));
    }

    if failures.len() > 5 {
        messages.push(format!("... and {} more", failures.len() - 5));
    }

    messages.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::types::CodeLocation;

    #[test]
    fn test_stage_execution_error() {
        let err = VerificationError::stage_execution("clippy", "lint errors", Some(1));
        assert!(err.to_string().contains("clippy"));
        assert!(err.to_string().contains("lint errors"));
        assert_eq!(err.stage_name(), Some("clippy"));
    }

    #[test]
    fn test_repair_generation_error() {
        let err = VerificationError::repair_generation(FailureType::Fmt, "Ollama unavailable");
        assert!(err.to_string().contains("fmt"));
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_repair_application_error() {
        let err = VerificationError::repair_application("src/main.rs", "file not found");
        assert!(err.to_string().contains("src/main.rs"));
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_stage_limit_exceeded() {
        let err = VerificationError::stage_limit_exceeded("fmt", 3, 2);
        assert!(err.to_string().contains("3/2"));
        assert!(err.is_limit_exceeded());
    }

    #[test]
    fn test_global_limit_exceeded() {
        let err = VerificationError::global_limit_exceeded(9, 8);
        assert!(err.to_string().contains("9/8"));
        assert!(err.is_limit_exceeded());
    }

    #[test]
    fn test_timeout_error() {
        let err = VerificationError::timeout("test", 300);
        assert!(err.is_timeout());
        assert_eq!(err.stage_name(), Some("test"));
    }

    #[test]
    fn test_privacy_violation() {
        let err = VerificationError::privacy_violation("contains file paths");
        assert!(err.to_string().contains("Privacy guard"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(VerificationError::stage_execution("fmt", "error", None).is_recoverable());
        assert!(!VerificationError::global_limit_exceeded(9, 8).is_recoverable());
        assert!(!VerificationError::NoStagesConfigured.is_recoverable());
    }

    #[test]
    fn test_aggregate_failures_empty() {
        let result = aggregate_failures(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_aggregate_failures_single() {
        let loc = CodeLocation::single_line("src/main.rs", 10);
        let failures = vec![StageFailure::fmt(loc, "formatting issue".to_string())];
        let result = aggregate_failures(&failures);
        assert!(result.contains("src/main.rs:10"));
        assert!(result.contains("formatting issue"));
    }

    #[test]
    fn test_aggregate_failures_many() {
        let failures: Vec<StageFailure> = (0..10)
            .map(|i| {
                let loc = CodeLocation::single_line("src/lib.rs", i + 1);
                StageFailure::fmt(loc, format!("error {}", i))
            })
            .collect();
        let result = aggregate_failures(&failures);
        // Should show first 5 and indicate more
        assert!(result.contains("1."));
        assert!(result.contains("5."));
        assert!(result.contains("and 5 more"));
    }
}
