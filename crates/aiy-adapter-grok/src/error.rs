//! Error types for the Grok adapter

use aiy_adapters::{AdapterError, AdapterErrorKind};
use thiserror::Error;

/// Errors that can occur when using the Grok adapter
#[derive(Debug, Error)]
pub enum GrokError {
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

impl From<aiy_core::security::SecurityError> for GrokError {
    fn from(err: aiy_core::security::SecurityError) -> Self {
        GrokError::Credential(err.to_string())
    }
}

impl From<aiy_core::security::sanitization::SanitizationError> for GrokError {
    fn from(err: aiy_core::security::sanitization::SanitizationError) -> Self {
        GrokError::SecurityValidation(err.to_string())
    }
}

impl GrokError {
    /// Get the error kind for this error.
    ///
    /// Maps internal error variants to the standardized `AdapterErrorKind`.
    pub fn kind(&self) -> AdapterErrorKind {
        match self {
            GrokError::Credential(_) => AdapterErrorKind::Auth,
            GrokError::ApiRequest(_) => AdapterErrorKind::Network,
            GrokError::RateLimit(_) => AdapterErrorKind::RateLimit,
            GrokError::ResponseParsing(_) => AdapterErrorKind::Parse,
            GrokError::SecurityValidation(_) => AdapterErrorKind::Security,
            GrokError::SchemaValidation(_) => AdapterErrorKind::Schema,
            GrokError::Transport(_) => AdapterErrorKind::Network,
            GrokError::Timeout(_) => AdapterErrorKind::Timeout,
            GrokError::Serialization(_) => AdapterErrorKind::Parse,
            GrokError::Other(_) => AdapterErrorKind::Unknown,
        }
    }

    /// Convert to a sanitized string that never exposes API keys or secrets.
    ///
    /// This method is used when converting to `AdapterError` for external consumers.
    pub fn to_sanitized_string(&self) -> String {
        match self {
            // Credential errors - never expose the actual credential value
            GrokError::Credential(_) => "Credential retrieval failed".to_string(),
            // API errors - sanitize to avoid leaking keys in URLs/headers
            GrokError::ApiRequest(_) => "API request failed".to_string(),
            GrokError::RateLimit(_) => "Rate limit exceeded".to_string(),
            // These are generally safe to expose
            GrokError::ResponseParsing(msg) => format!("Response parsing failed: {msg}"),
            GrokError::SecurityValidation(msg) => format!("Security validation failed: {msg}"),
            GrokError::SchemaValidation(msg) => format!("Schema validation failed: {msg}"),
            GrokError::Transport(_) => "Transport error occurred".to_string(),
            GrokError::Timeout(_) => "Request timed out".to_string(),
            GrokError::Serialization(_) => "Serialization error".to_string(),
            GrokError::Other(msg) => msg.clone(),
        }
    }

    /// Convert to an `AdapterError` with appropriate kind and sanitized message.
    pub fn to_adapter_error(&self) -> AdapterError {
        AdapterError::new(self.kind(), self.to_sanitized_string())
    }
}
