//! Error types for the Codex adapter

use aiy_adapters::{AdapterError, AdapterErrorKind};
use thiserror::Error;

/// Errors that can occur when using the Codex adapter
#[derive(Debug, Error)]
pub enum CodexError {
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

impl From<aiy_core::security::SecurityError> for CodexError {
    fn from(err: aiy_core::security::SecurityError) -> Self {
        CodexError::Credential(err.to_string())
    }
}

impl From<aiy_core::security::sanitization::SanitizationError> for CodexError {
    fn from(err: aiy_core::security::sanitization::SanitizationError) -> Self {
        CodexError::SecurityValidation(err.to_string())
    }
}

impl CodexError {
    /// Get the error kind for this error.
    ///
    /// Maps internal error variants to the standardized `AdapterErrorKind`.
    pub fn kind(&self) -> AdapterErrorKind {
        match self {
            CodexError::Credential(_) => AdapterErrorKind::Auth,
            CodexError::ApiRequest(_) => AdapterErrorKind::Network,
            CodexError::RateLimit(_) => AdapterErrorKind::RateLimit,
            CodexError::ResponseParsing(_) => AdapterErrorKind::Parse,
            CodexError::SecurityValidation(_) => AdapterErrorKind::Security,
            CodexError::SchemaValidation(_) => AdapterErrorKind::Schema,
            CodexError::Transport(_) => AdapterErrorKind::Network,
            CodexError::Timeout(_) => AdapterErrorKind::Timeout,
            CodexError::Serialization(_) => AdapterErrorKind::Parse,
            CodexError::Other(_) => AdapterErrorKind::Unknown,
        }
    }

    /// Convert to a sanitized string that never exposes API keys or secrets.
    ///
    /// This method is used when converting to `AdapterError` for external consumers.
    pub fn to_sanitized_string(&self) -> String {
        match self {
            // Credential errors - never expose the actual credential value
            CodexError::Credential(_) => "Credential retrieval failed".to_string(),
            // API errors - sanitize to avoid leaking keys in URLs/headers
            CodexError::ApiRequest(_) => "API request failed".to_string(),
            CodexError::RateLimit(_) => "Rate limit exceeded".to_string(),
            // These are generally safe to expose
            CodexError::ResponseParsing(msg) => format!("Response parsing failed: {msg}"),
            CodexError::SecurityValidation(msg) => format!("Security validation failed: {msg}"),
            CodexError::SchemaValidation(msg) => format!("Schema validation failed: {msg}"),
            CodexError::Transport(_) => "Transport error occurred".to_string(),
            CodexError::Timeout(_) => "Request timed out".to_string(),
            CodexError::Serialization(_) => "Serialization error".to_string(),
            CodexError::Other(msg) => msg.clone(),
        }
    }

    /// Convert to an `AdapterError` with appropriate kind and sanitized message.
    pub fn to_adapter_error(&self) -> AdapterError {
        AdapterError::new(self.kind(), self.to_sanitized_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_error_sanitized() {
        let err = CodexError::Credential("secret-api-key-12345".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Credential retrieval failed");
        assert!(!sanitized.contains("secret"));
        assert!(!sanitized.contains("12345"));
    }

    #[test]
    fn test_api_request_error_sanitized() {
        let err = CodexError::ApiRequest("Bearer sk-12345 in header".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "API request failed");
        assert!(!sanitized.contains("Bearer"));
        assert!(!sanitized.contains("sk-12345"));
    }

    #[test]
    fn test_transport_error_sanitized() {
        let err = CodexError::Transport("connection failed with auth header".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Transport error occurred");
        assert!(!sanitized.contains("auth"));
    }

    #[test]
    fn test_response_parsing_preserved() {
        let err = CodexError::ResponseParsing("invalid JSON at position 42".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Response parsing failed: invalid JSON at position 42");
    }

    #[test]
    fn test_security_validation_preserved() {
        let err = CodexError::SecurityValidation("suspicious pattern detected".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(
            sanitized,
            "Security validation failed: suspicious pattern detected"
        );
    }

    #[test]
    fn test_schema_validation_preserved() {
        let err = CodexError::SchemaValidation("missing field 'verdict'".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Schema validation failed: missing field 'verdict'");
    }

    #[test]
    fn test_error_display() {
        let err = CodexError::Credential("test".to_string());
        assert_eq!(err.to_string(), "Credential error: test");
    }
}
