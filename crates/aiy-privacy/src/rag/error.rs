//! Error types for the local RAG system.
//!
//! This module defines all error types that can occur during RAG operations.
//! Errors are designed to be informative without leaking sensitive information.

use thiserror::Error;

/// Errors that can occur during RAG operations.
#[derive(Debug, Error)]
pub enum RagError {
    /// Embedding generation failed
    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),

    /// Embedding model not available
    #[error("Embedding model not available: {0}")]
    ModelNotAvailable(String),

    /// Connection to embedding service failed
    #[error("Connection to embedding service failed: {0}")]
    ConnectionFailed(String),

    /// Chunk not found in store
    #[error("Chunk not found: {0}")]
    ChunkNotFound(String),

    /// Invalid embedding dimension
    #[error("Invalid embedding dimension: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Token budget exceeded
    #[error("Token budget exceeded: {used} tokens used, budget is {budget}")]
    TokenBudgetExceeded { used: usize, budget: usize },

    /// No chunks available for query
    #[error("No chunks available for query")]
    NoChunksAvailable,

    /// Indexing error (generic, path info sanitized)
    #[error("Indexing error: {0}")]
    IndexingError(String),

    /// File read error (path sanitized)
    #[error("Failed to read file: {0}")]
    FileReadError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Chunking error
    #[error("Chunking error: {0}")]
    ChunkingError(String),

    /// Query error
    #[error("Query error: {0}")]
    QueryError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Request timeout
    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    /// Transport error
    #[error("Transport error: {0}")]
    TransportError(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {0} seconds")]
    RateLimited(u64),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl RagError {
    /// Check if this error is transient and the operation can be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            RagError::ConnectionFailed(_)
                | RagError::Timeout(_)
                | RagError::TransportError(_)
                | RagError::RateLimited(_)
        )
    }

    /// Create an indexing error with a sanitized message.
    ///
    /// This ensures no file paths are leaked in error messages.
    pub fn indexing_sanitized(msg: impl Into<String>) -> Self {
        RagError::IndexingError(sanitize_error_message(&msg.into()))
    }

    /// Create a file read error with a sanitized message.
    pub fn file_read_sanitized(msg: impl Into<String>) -> Self {
        RagError::FileReadError(sanitize_error_message(&msg.into()))
    }

    /// Convert to a user-facing message that never exposes sensitive info.
    pub fn to_user_message(&self) -> String {
        match self {
            RagError::EmbeddingFailed(_) => {
                "Failed to generate embeddings. Check if Ollama is running.".to_string()
            }
            RagError::ModelNotAvailable(model) => {
                format!(
                    "Embedding model '{}' not available. Run: ollama pull {}",
                    model, model
                )
            }
            RagError::ConnectionFailed(_) => {
                "Failed to connect to local embedding service. Is Ollama running?".to_string()
            }
            RagError::ChunkNotFound(_) => "Requested code chunk not found.".to_string(),
            RagError::DimensionMismatch { expected, actual } => {
                format!("Embedding dimension mismatch: expected {expected}, got {actual}")
            }
            RagError::TokenBudgetExceeded { used, budget } => {
                format!("Context too large: {used} tokens exceeds budget of {budget}")
            }
            RagError::NoChunksAvailable => "No code indexed for search.".to_string(),
            RagError::IndexingError(_) => "Failed to index code.".to_string(),
            RagError::FileReadError(_) => "Failed to read source file.".to_string(),
            RagError::InvalidConfig(msg) => format!("Invalid configuration: {msg}"),
            RagError::ChunkingError(_) => "Failed to chunk code.".to_string(),
            RagError::QueryError(_) => "Query processing failed.".to_string(),
            RagError::SerializationError(_) => "Serialization error.".to_string(),
            RagError::Timeout(ms) => format!("Request timed out after {}ms", ms),
            RagError::TransportError(_) => "Transport error occurred.".to_string(),
            RagError::RateLimited(secs) => format!("Rate limited. Retry after {}s", secs),
            RagError::Internal(_) => "Internal error occurred.".to_string(),
        }
    }
}

/// Sanitize an error message to remove potential file paths.
fn sanitize_error_message(msg: &str) -> String {
    // Remove anything that looks like a file path
    let sanitized = regex::Regex::new(r"[/\\][\w./-]+")
        .map(|re| re.replace_all(msg, "[path]").to_string())
        .unwrap_or_else(|_| msg.to_string());

    // Also remove home directory patterns
    let sanitized = regex::Regex::new(r"~[\w./-]*")
        .map(|re| re.replace_all(&sanitized, "[path]").to_string())
        .unwrap_or(sanitized);

    sanitized
}

impl From<std::io::Error> for RagError {
    fn from(err: std::io::Error) -> Self {
        // Sanitize the error message to avoid leaking paths
        RagError::FileReadError(sanitize_error_message(&err.to_string()))
    }
}

impl From<serde_json::Error> for RagError {
    fn from(err: serde_json::Error) -> Self {
        RagError::SerializationError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable() {
        assert!(RagError::ConnectionFailed("test".to_string()).is_retryable());
        assert!(RagError::Timeout(1000).is_retryable());
        assert!(RagError::RateLimited(60).is_retryable());

        assert!(!RagError::ChunkNotFound("test".to_string()).is_retryable());
        assert!(!RagError::InvalidConfig("test".to_string()).is_retryable());
    }

    #[test]
    fn test_sanitize_error_message() {
        let msg = "Error reading /home/user/project/src/main.rs";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("/home"));
        assert!(!sanitized.contains("main.rs"));
        assert!(sanitized.contains("[path]"));
    }

    #[test]
    fn test_sanitize_windows_paths() {
        let msg = r"Error reading C:\Users\user\project\src\main.rs";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("Users"));
        assert!(!sanitized.contains("main.rs"));
    }

    #[test]
    fn test_sanitize_home_dir() {
        let msg = "Error reading ~/project/src/main.rs";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("~"));
        assert!(!sanitized.contains("project"));
    }

    #[test]
    fn test_user_message_no_paths() {
        let err = RagError::indexing_sanitized("Failed to read /home/user/secret/file.rs");
        let user_msg = err.to_user_message();
        assert!(!user_msg.contains("/home"));
        assert!(!user_msg.contains("secret"));
        assert!(!user_msg.contains("file.rs"));
    }

    #[test]
    fn test_model_not_available_message() {
        let err = RagError::ModelNotAvailable("nomic-embed-text".to_string());
        let msg = err.to_user_message();
        assert!(msg.contains("nomic-embed-text"));
        assert!(msg.contains("ollama pull"));
    }

    #[test]
    fn test_token_budget_exceeded_message() {
        let err = RagError::TokenBudgetExceeded {
            used: 5000,
            budget: 2048,
        };
        let msg = err.to_user_message();
        assert!(msg.contains("5000"));
        assert!(msg.contains("2048"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No such file: /home/user/file.txt",
        );
        let rag_err: RagError = io_err.into();
        match rag_err {
            RagError::FileReadError(msg) => {
                assert!(!msg.contains("/home"));
            }
            _ => panic!("Expected FileReadError"),
        }
    }
}
