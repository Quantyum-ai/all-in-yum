//! Error types for the Claude adapter

use aiy_adapters::{AdapterError, AdapterErrorKind};
use thiserror::Error;

/// Errors that can occur when using the Claude adapter
#[derive(Debug, Error)]
pub enum ClaudeError {
    /// Credential retrieval failed
    #[error("Credential error: {0}")]
    Credential(String),

    /// API request failed
    #[error("API request failed: {0}")]
    ApiRequest(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// API response parsing failed
    #[error("Response parsing failed: {0}")]
    ResponseParsing(String),

    /// Prompt injection defense validation failed
    #[error("Security validation failed: {0}")]
    SecurityValidation(String),

    /// Schema validation failed
    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    /// Transport error (mock or real HTTP)
    #[error("Transport error: {0}")]
    Transport(String),

    /// Request timeout
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<aiy_core::security::SecurityError> for ClaudeError {
    fn from(err: aiy_core::security::SecurityError) -> Self {
        ClaudeError::Credential(err.to_string())
    }
}

impl From<aiy_core::security::sanitization::SanitizationError> for ClaudeError {
    fn from(err: aiy_core::security::sanitization::SanitizationError) -> Self {
        ClaudeError::SecurityValidation(err.to_string())
    }
}

impl ClaudeError {
    /// Get the error kind for this error.
    ///
    /// Maps internal error variants to the standardized `AdapterErrorKind`.
    pub fn kind(&self) -> AdapterErrorKind {
        match self {
            ClaudeError::Credential(_) => AdapterErrorKind::Auth,
            ClaudeError::ApiRequest(_) => AdapterErrorKind::Network,
            ClaudeError::RateLimit(_) => AdapterErrorKind::RateLimit,
            ClaudeError::ResponseParsing(_) => AdapterErrorKind::Parse,
            ClaudeError::SecurityValidation(_) => AdapterErrorKind::Security,
            ClaudeError::SchemaValidation(_) => AdapterErrorKind::Schema,
            ClaudeError::Transport(_) => AdapterErrorKind::Network,
            ClaudeError::Timeout(_) => AdapterErrorKind::Timeout,
            ClaudeError::Serialization(_) => AdapterErrorKind::Parse,
            ClaudeError::Other(_) => AdapterErrorKind::Unknown,
        }
    }

    /// Convert to a sanitized string that never exposes API keys or secrets.
    ///
    /// This method is used when converting to `AdapterError` for external consumers.
    pub fn to_sanitized_string(&self) -> String {
        match self {
            // Credential errors - never expose the actual credential value
            ClaudeError::Credential(_) => "Credential retrieval failed".to_string(),
            // API errors - sanitize to avoid leaking keys in URLs/headers
            ClaudeError::ApiRequest(_) => "API request failed".to_string(),
            ClaudeError::RateLimit(_) => "Rate limit exceeded".to_string(),
            // These are generally safe to expose
            ClaudeError::ResponseParsing(msg) => format!("Response parsing failed: {msg}"),
            ClaudeError::SecurityValidation(msg) => format!("Security validation failed: {msg}"),
            ClaudeError::SchemaValidation(msg) => format!("Schema validation failed: {msg}"),
            ClaudeError::Transport(_) => "Transport error occurred".to_string(),
            ClaudeError::Timeout(_) => "Request timed out".to_string(),
            ClaudeError::Serialization(_) => "Serialization error".to_string(),
            ClaudeError::Other(msg) => msg.clone(),
        }
    }

    /// Convert to an `AdapterError` with appropriate kind and sanitized message.
    pub fn to_adapter_error(&self) -> AdapterError {
        AdapterError::new(self.kind(), self.to_sanitized_string())
    }
}
