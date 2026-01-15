//! Orchestration error types
//!
//! Defines errors that can occur during privacy mode orchestration.

use thiserror::Error;

/// Errors that can occur during orchestration
#[derive(Debug, Error)]
pub enum OrchestrationError {
    /// Session not initialized
    #[error("Orchestration session not initialized. Call init() first.")]
    SessionNotInitialized,

    /// Session already exists
    #[error("Orchestration session already exists")]
    SessionAlreadyExists,

    /// Invalid session state
    #[error("Invalid session state: {0}")]
    InvalidState(String),

    /// Cloud communication error
    #[error("Cloud communication error: {0}")]
    CloudError(String),

    /// Local execution error
    #[error("Local execution error: {0}")]
    LocalError(String),

    /// Plan parsing error
    #[error("Failed to parse plan: {0}")]
    PlanParsingError(String),

    /// Task execution failed
    #[error("Task execution failed: {task_id} - {reason}")]
    TaskFailed {
        /// Task identifier (opaque)
        task_id: String,
        /// Reason for failure
        reason: String,
    },

    /// Verification failed after max attempts
    #[error("Verification failed after {attempts} repair attempts")]
    VerificationFailed {
        /// Number of attempts made
        attempts: usize,
    },

    /// Privacy violation detected
    #[error("Privacy violation: content contains sensitive information")]
    PrivacyViolation,

    /// RAG system error
    #[error("RAG system error: {0}")]
    RagError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Cancelled by user
    #[error("Orchestration cancelled")]
    Cancelled,
}

impl OrchestrationError {
    /// Create a cloud communication error
    pub fn cloud(msg: impl Into<String>) -> Self {
        Self::CloudError(msg.into())
    }

    /// Create a local execution error
    pub fn local(msg: impl Into<String>) -> Self {
        Self::LocalError(msg.into())
    }

    /// Create a plan parsing error
    pub fn plan_parsing(msg: impl Into<String>) -> Self {
        Self::PlanParsingError(msg.into())
    }

    /// Create a task failed error
    pub fn task_failed(task_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::TaskFailed {
            task_id: task_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a verification failed error
    pub fn verification_failed(attempts: usize) -> Self {
        Self::VerificationFailed { attempts }
    }

    /// Create a RAG error
    pub fn rag(msg: impl Into<String>) -> Self {
        Self::RagError(msg.into())
    }

    /// Create a config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }
}

/// Result type for orchestration operations
pub type OrchestrationResult<T> = Result<T, OrchestrationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = OrchestrationError::SessionNotInitialized;
        assert!(err.to_string().contains("not initialized"));

        let err = OrchestrationError::task_failed("TASK_001", "compile error");
        assert!(err.to_string().contains("TASK_001"));
        assert!(err.to_string().contains("compile error"));
    }

    #[test]
    fn test_error_constructors() {
        let _ = OrchestrationError::cloud("connection failed");
        let _ = OrchestrationError::local("ollama timeout");
        let _ = OrchestrationError::plan_parsing("invalid JSON");
        let _ = OrchestrationError::verification_failed(3);
        let _ = OrchestrationError::rag("embedding failed");
        let _ = OrchestrationError::config("invalid url");
    }
}
