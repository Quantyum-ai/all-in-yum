//! Error types for the Ollama adapter

use aiy_adapters::{AdapterError, AdapterErrorKind};
use thiserror::Error;

/// Errors that can occur when using the Ollama adapter
#[derive(Debug, Error)]
pub enum OllamaError {
    /// Configuration error (invalid URL, missing model, etc.)
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Connection to Ollama server failed
    #[error("Connection failed: {0}")]
    Connection(String),

    /// Model not found or not available
    #[error("Model not available: {0}")]
    ModelNotAvailable(String),

    /// API request failed
    #[error("API request failed: {0}")]
    ApiRequest(String),

    /// Rate limit or resource exhaustion
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Response parsing failed
    #[error("Response parsing failed: {0}")]
    ResponseParsing(String),

    /// Security validation failed
    #[error("Security validation failed: {0}")]
    SecurityValidation(String),

    /// Schema validation failed
    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    /// Transport error (HTTP/mock)
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

impl OllamaError {
    /// Check if this error is retryable.
    ///
    /// Returns true for transient errors like timeouts and transport errors.
    /// Returns false for permanent errors like config or parsing errors.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ResourceExhausted(_) | Self::Timeout(_) | Self::Transport(_)
        )
    }

    /// Get the error kind for this error.
    ///
    /// Maps internal error variants to the standardized `AdapterErrorKind`.
    pub fn kind(&self) -> AdapterErrorKind {
        match self {
            OllamaError::Configuration(_) => AdapterErrorKind::Schema,
            OllamaError::Connection(_) => AdapterErrorKind::Network,
            OllamaError::ModelNotAvailable(_) => AdapterErrorKind::Schema,
            OllamaError::ApiRequest(_) => AdapterErrorKind::Network,
            OllamaError::ResourceExhausted(_) => AdapterErrorKind::RateLimit,
            OllamaError::ResponseParsing(_) => AdapterErrorKind::Parse,
            OllamaError::SecurityValidation(_) => AdapterErrorKind::Security,
            OllamaError::SchemaValidation(_) => AdapterErrorKind::Schema,
            OllamaError::Transport(_) => AdapterErrorKind::Network,
            OllamaError::Timeout(_) => AdapterErrorKind::Timeout,
            OllamaError::Serialization(_) => AdapterErrorKind::Parse,
            OllamaError::Other(_) => AdapterErrorKind::Unknown,
        }
    }

    /// Convert to a sanitized string that never exposes sensitive information.
    ///
    /// For Ollama (local), there are no API keys, but we still sanitize
    /// to prevent leaking file paths or internal details.
    pub fn to_sanitized_string(&self) -> String {
        match self {
            OllamaError::Configuration(_) => "Configuration error".to_string(),
            OllamaError::Connection(_) => "Failed to connect to Ollama server".to_string(),
            OllamaError::ModelNotAvailable(model) => {
                format!("Model '{}' not available. Run: ollama pull {}", model, model)
            }
            OllamaError::ApiRequest(_) => "API request failed".to_string(),
            OllamaError::ResourceExhausted(_) => "Server resources exhausted".to_string(),
            OllamaError::ResponseParsing(msg) => format!("Response parsing failed: {msg}"),
            OllamaError::SecurityValidation(msg) => format!("Security validation failed: {msg}"),
            OllamaError::SchemaValidation(msg) => format!("Schema validation failed: {msg}"),
            OllamaError::Transport(_) => "Transport error occurred".to_string(),
            OllamaError::Timeout(_) => "Request timed out".to_string(),
            OllamaError::Serialization(_) => "Serialization error".to_string(),
            OllamaError::Other(msg) => msg.clone(),
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
    fn test_connection_error_is_not_retryable_by_default() {
        // Connection errors might be transient but we don't retry them by default
        let err = OllamaError::Connection("refused".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_timeout_is_retryable() {
        let err = OllamaError::Timeout("30s exceeded".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_transport_is_retryable() {
        let err = OllamaError::Transport("connection reset".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_resource_exhausted_is_retryable() {
        let err = OllamaError::ResourceExhausted("GPU memory full".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_config_error_not_retryable() {
        let err = OllamaError::Configuration("invalid URL".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_model_not_available_provides_helpful_message() {
        let err = OllamaError::ModelNotAvailable("codellama:7b".to_string());
        let sanitized = err.to_sanitized_string();
        assert!(sanitized.contains("ollama pull"));
        assert!(sanitized.contains("codellama:7b"));
    }

    #[test]
    fn test_error_kind_mapping() {
        assert_eq!(
            OllamaError::Connection("test".to_string()).kind(),
            AdapterErrorKind::Network
        );
        assert_eq!(
            OllamaError::Timeout("test".to_string()).kind(),
            AdapterErrorKind::Timeout
        );
        assert_eq!(
            OllamaError::ResourceExhausted("test".to_string()).kind(),
            AdapterErrorKind::RateLimit
        );
    }
}
