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
