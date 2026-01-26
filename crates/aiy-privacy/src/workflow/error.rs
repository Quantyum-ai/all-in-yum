//! Workflow error types
//!
//! Defines errors specific to workflow operations.

use thiserror::Error;

/// Errors that can occur during workflow operations
#[derive(Debug, Error)]
pub enum WorkflowError {
    /// Workflow not initialized
    #[error("Workflow not initialized. Run `aiy privacy init` first.")]
    NotInitialized,

    /// Workflow already initialized
    #[error("Workflow already initialized in this directory")]
    AlreadyInitialized,

    /// Invalid workflow state
    #[error("Invalid workflow state: {0}")]
    InvalidState(String),

    /// Orchestration error
    #[error("Orchestration error: {0}")]
    OrchestrationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Cancelled
    #[error("Workflow cancelled")]
    Cancelled,
}

impl WorkflowError {
    /// Create an orchestration error
    pub fn orchestration(msg: impl Into<String>) -> Self {
        Self::OrchestrationError(msg.into())
    }

    /// Create a serialization error
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::SerializationError(msg.into())
    }

    /// Create a config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }
}

/// Result type for workflow operations
pub type WorkflowResult<T> = Result<T, WorkflowError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = WorkflowError::NotInitialized;
        assert!(err.to_string().contains("not initialized"));

        let err = WorkflowError::orchestration("failed");
        assert!(err.to_string().contains("failed"));
    }
}
