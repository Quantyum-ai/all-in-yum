//! Error types for the Gemini adapter

use aiy_adapters::{AdapterError, AdapterErrorKind};
use thiserror::Error;

/// Errors that can occur when using the Gemini adapter
#[derive(Debug, Error)]
pub enum GeminiError {
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

impl From<aiy_core::security::SecurityError> for GeminiError {
    fn from(err: aiy_core::security::SecurityError) -> Self {
        GeminiError::Credential(err.to_string())
    }
}

impl From<aiy_core::security::sanitization::SanitizationError> for GeminiError {
    fn from(err: aiy_core::security::sanitization::SanitizationError) -> Self {
        GeminiError::SecurityValidation(err.to_string())
    }
}

impl GeminiError {
    /// Get the error kind for this error.
    ///
    /// Maps internal error variants to the standardized `AdapterErrorKind`.
    pub fn kind(&self) -> AdapterErrorKind {
        match self {
            GeminiError::Credential(_) => AdapterErrorKind::Auth,
            GeminiError::ApiRequest(_) => AdapterErrorKind::Network,
            GeminiError::RateLimit(_) => AdapterErrorKind::RateLimit,
            GeminiError::ResponseParsing(_) => AdapterErrorKind::Parse,
            GeminiError::SecurityValidation(_) => AdapterErrorKind::Security,
            GeminiError::SchemaValidation(_) => AdapterErrorKind::Schema,
            GeminiError::Transport(_) => AdapterErrorKind::Network,
            GeminiError::Timeout(_) => AdapterErrorKind::Timeout,
            GeminiError::Serialization(_) => AdapterErrorKind::Parse,
            GeminiError::Other(_) => AdapterErrorKind::Unknown,
        }
    }

    /// Convert to a sanitized string that never exposes API keys or secrets.
    ///
    /// This method is used when converting to `AdapterError` for external consumers.
    /// CRITICAL: Query-string API keys must NEVER appear in error messages.
    pub fn to_sanitized_string(&self) -> String {
        match self {
            // Credential errors - never expose the actual credential value
            GeminiError::Credential(_) => "Credential retrieval failed".to_string(),
            // API errors - sanitize to avoid leaking keys in URLs (query string!)
            GeminiError::ApiRequest(_) => "API request failed".to_string(),
            GeminiError::RateLimit(_) => "Rate limit exceeded".to_string(),
            // Transport errors may contain URLs with API keys - sanitize them
            GeminiError::Transport(_) => "Transport error occurred".to_string(),
            GeminiError::Timeout(_) => "Request timed out".to_string(),
            // These are generally safe to expose
            GeminiError::ResponseParsing(msg) => format!("Response parsing failed: {msg}"),
            GeminiError::SecurityValidation(msg) => format!("Security validation failed: {msg}"),
            GeminiError::SchemaValidation(msg) => format!("Schema validation failed: {msg}"),
            GeminiError::Serialization(_) => "Serialization error".to_string(),
            GeminiError::Other(msg) => msg.clone(),
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
    fn test_credential_error_sanitization() {
        let err = GeminiError::Credential("API key: AIzaSyC123456789abcdef".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Credential retrieval failed");
        assert!(!sanitized.contains("AIzaSy"));
    }

    #[test]
    fn test_api_request_error_sanitization() {
        let err = GeminiError::ApiRequest(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent?key=AIzaSyC123456789".to_string()
        );
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "API request failed");
        assert!(!sanitized.contains("AIzaSy"));
        assert!(!sanitized.contains("key="));
    }

    #[test]
    fn test_transport_error_sanitization() {
        let err = GeminiError::Transport(
            "Connection failed to URL with key=SECRET123".to_string()
        );
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Transport error occurred");
        assert!(!sanitized.contains("SECRET"));
        assert!(!sanitized.contains("key="));
    }

    #[test]
    fn test_safe_errors_pass_through() {
        let err = GeminiError::ResponseParsing("Invalid JSON at line 5".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Response parsing failed: Invalid JSON at line 5");

        let err = GeminiError::SecurityValidation("Suspicious pattern detected".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Security validation failed: Suspicious pattern detected");

        let err = GeminiError::SchemaValidation("Missing required field 'verdict'".to_string());
        let sanitized = err.to_sanitized_string();
        assert_eq!(sanitized, "Schema validation failed: Missing required field 'verdict'");
    }
}
