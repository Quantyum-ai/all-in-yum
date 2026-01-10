//! Error types for the Grok adapter

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
    /// Convert to a sanitized string that never exposes API keys or secrets.
    ///
    /// This method is used when converting to `AdapterError` for external consumers.
    pub fn to_sanitized_string(&self) -> String {
        match self {
            // Credential errors - never expose the actual credential value
            GrokError::Credential(_) => "Credential retrieval failed".to_string(),
            // API errors - sanitize to avoid leaking keys in URLs/headers
            GrokError::ApiRequest(_) => "API request failed".to_string(),
            // These are generally safe to expose
            GrokError::ResponseParsing(msg) => format!("Response parsing failed: {msg}"),
            GrokError::SecurityValidation(msg) => format!("Security validation failed: {msg}"),
            GrokError::SchemaValidation(msg) => format!("Schema validation failed: {msg}"),
            GrokError::Transport(_) => "Transport error occurred".to_string(),
            GrokError::Serialization(_) => "Serialization error".to_string(),
            GrokError::Other(msg) => msg.clone(),
        }
    }
}
