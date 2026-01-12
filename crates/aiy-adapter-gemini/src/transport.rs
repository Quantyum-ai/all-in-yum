//! HTTP transport abstraction for Gemini API
//!
//! Provides both mock and real HTTP transports for testing and production use.
//!
//! SECURITY NOTE: Gemini uses API keys in the query string, NOT headers.
//! This means URLs must NEVER be logged or included in error messages.

use crate::error::GeminiError;
use async_trait::async_trait;
use std::sync::Mutex;

/// Type alias for mock response generator functions
type MockResponseFn = Box<dyn Fn(&str) -> Result<String, GeminiError> + Send + Sync>;

/// HTTP transport trait for making API requests
///
/// NOTE: For Gemini, the URL will include the API key as a query parameter.
/// Implementations must ensure the key is never logged.
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Send a POST request with JSON body
    ///
    /// SECURITY: The URL may contain API keys in query parameters.
    /// Implementations MUST NOT log URLs or include them in error messages.
    async fn post_json(&self, url: &str, body: &str) -> Result<String, GeminiError>;
}

/// Mock transport for testing
pub struct MockTransport {
    /// Canned response to return
    response: Mutex<Option<String>>,
    /// Custom response function
    response_fn: Option<MockResponseFn>,
}

impl MockTransport {
    /// Create a new mock transport with no canned response
    pub fn new() -> Self {
        Self {
            response: Mutex::new(None),
            response_fn: None,
        }
    }

    /// Create a mock transport with a canned response
    pub fn with_canned_response(response: String) -> Self {
        Self {
            response: Mutex::new(Some(response)),
            response_fn: None,
        }
    }

    /// Create a mock transport with a custom response function
    pub fn with_response<F>(f: F) -> Self
    where
        F: Fn(&str) -> Result<String, GeminiError> + Send + Sync + 'static,
    {
        Self {
            response: Mutex::new(None),
            response_fn: Some(Box::new(f)),
        }
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpTransport for MockTransport {
    async fn post_json(&self, _url: &str, body: &str) -> Result<String, GeminiError> {
        // If we have a custom response function, use it
        if let Some(ref f) = self.response_fn {
            return f(body);
        }

        // Otherwise, return the canned response
        let response = self.response.lock().unwrap();
        response
            .clone()
            .ok_or_else(|| GeminiError::Transport("No mock response configured".to_string()))
    }
}

/// Real HTTP transport using reqwest (feature-gated)
#[cfg(feature = "http")]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

#[cfg(feature = "http")]
impl ReqwestTransport {
    /// Create a new reqwest transport with default settings
    pub fn new() -> Result<Self, GeminiError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| GeminiError::Transport(e.to_string()))?;
        Ok(Self { client })
    }

    /// Create a new reqwest transport with custom timeout
    pub fn with_timeout(timeout: std::time::Duration) -> Result<Self, GeminiError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| GeminiError::Transport(e.to_string()))?;
        Ok(Self { client })
    }

    /// Execute a single HTTP request (without retry logic)
    async fn execute_request(&self, url: &str, body: &str) -> Result<String, GeminiError> {
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| GeminiError::Transport(e.to_string()))?;

        let status = response.status();

        if status.as_u16() == 429 {
            return Err(GeminiError::RateLimit("Rate limit exceeded".to_string()));
        }
        if status.is_server_error() {
            return Err(GeminiError::Transport(format!("Server error: {}", status)));
        }
        if !status.is_success() {
            // SECURITY: Do NOT include URL in error message (contains API key)
            return Err(GeminiError::ApiRequest(format!("HTTP {}", status)));
        }

        response
            .text()
            .await
            .map_err(|e| GeminiError::Transport(e.to_string()))
    }
}

#[cfg(feature = "http")]
#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn post_json(&self, url: &str, body: &str) -> Result<String, GeminiError> {
        use aiy_adapters::RetryConfig;

        let config = RetryConfig::default();

        for attempt in 0..=config.max_retries {
            match self.execute_request(url, body).await {
                Ok(text) => return Ok(text),
                Err(e) if attempt < config.max_retries && e.is_retryable() => {
                    tokio::time::sleep(config.calculate_delay(attempt)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_transport_canned_response() {
        let transport = MockTransport::with_canned_response("test response".to_string());
        let result = transport.post_json("http://test", "body").await;
        assert_eq!(result.unwrap(), "test response");
    }

    #[tokio::test]
    async fn test_mock_transport_no_response() {
        let transport = MockTransport::new();
        let result = transport.post_json("http://test", "body").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_transport_with_function() {
        let transport = MockTransport::with_response(|body| Ok(format!("echo: {}", body)));
        let result = transport.post_json("http://test", "hello").await;
        assert_eq!(result.unwrap(), "echo: hello");
    }
}
