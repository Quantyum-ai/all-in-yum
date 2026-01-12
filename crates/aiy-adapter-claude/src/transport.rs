//! HTTP transport abstraction for Claude API
//!
//! Provides both mock and real HTTP transports for testing and production use.

use crate::error::ClaudeError;
use async_trait::async_trait;
use std::sync::Mutex;
#[cfg(feature = "http")]
use std::time::Duration;

/// Type alias for mock response generator functions
type MockResponseFn = Box<dyn Fn(&str) -> Result<String, ClaudeError> + Send + Sync>;

/// HTTP transport trait for making API requests
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Send a POST request with JSON body
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, ClaudeError>;
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
        F: Fn(&str) -> Result<String, ClaudeError> + Send + Sync + 'static,
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
    async fn post_json(
        &self,
        _url: &str,
        _headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, ClaudeError> {
        // If we have a custom response function, use it
        if let Some(ref f) = self.response_fn {
            return f(body);
        }

        // Otherwise, return the canned response
        let response = self.response.lock().unwrap();
        response
            .clone()
            .ok_or_else(|| ClaudeError::Transport("No mock response configured".to_string()))
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
    pub fn new() -> Result<Self, ClaudeError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| ClaudeError::Transport(e.to_string()))?;
        Ok(Self { client })
    }

    /// Create a new reqwest transport with custom timeout
    pub fn with_timeout(timeout: std::time::Duration) -> Result<Self, ClaudeError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ClaudeError::Transport(e.to_string()))?;
        Ok(Self { client })
    }
}

#[cfg(feature = "http")]
impl ReqwestTransport {
    /// Execute a single HTTP request without retries
    async fn execute_request(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, ClaudeError> {
        let mut req = self.client.post(url).body(body.to_string());
        for (key, value) in headers {
            req = req.header(*key, *value);
        }

        let response = req
            .send()
            .await
            .map_err(|e| ClaudeError::Transport(sanitize_error_message(&e.to_string())))?;

        let status = response.status();

        // Classify errors for retry logic
        if status.as_u16() == 429 {
            return Err(ClaudeError::RateLimit("Rate limit exceeded".to_string()));
        }
        if status.is_server_error() {
            return Err(ClaudeError::Transport(format!("Server error: {}", status)));
        }
        if !status.is_success() {
            return Err(ClaudeError::ApiRequest(format!("HTTP {}", status)));
        }

        response
            .text()
            .await
            .map_err(|e| ClaudeError::Transport(sanitize_error_message(&e.to_string())))
    }
}

#[cfg(feature = "http")]
#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, ClaudeError> {
        // Retry logic with exponential backoff
        let delays = [
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
        ];
        let max_retries = delays.len();

        for attempt in 0..=max_retries {
            match self.execute_request(url, headers, body).await {
                Ok(text) => return Ok(text),
                Err(e) if attempt < max_retries && e.is_retryable() => {
                    tokio::time::sleep(delays[attempt]).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }
}

/// Sanitize error messages to prevent API key leakage
#[cfg(any(test, feature = "http"))]
fn sanitize_error_message(msg: &str) -> String {
    // Remove anything that looks like an API key
    let sanitized = regex::Regex::new(r"(sk-ant-[a-zA-Z0-9\-]+|Bearer [a-zA-Z0-9\-_]+)")
        .unwrap()
        .replace_all(msg, "[REDACTED]");
    sanitized.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_transport_canned_response() {
        let transport = MockTransport::with_canned_response("test response".to_string());
        let result = transport.post_json("http://test", &[], "body").await;
        assert_eq!(result.unwrap(), "test response");
    }

    #[tokio::test]
    async fn test_mock_transport_no_response() {
        let transport = MockTransport::new();
        let result = transport.post_json("http://test", &[], "body").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_transport_with_function() {
        let transport = MockTransport::with_response(|body| Ok(format!("echo: {}", body)));
        let result = transport.post_json("http://test", &[], "hello").await;
        assert_eq!(result.unwrap(), "echo: hello");
    }

    #[test]
    fn test_rate_limit_is_retryable() {
        let err = ClaudeError::RateLimit("Rate limit exceeded".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_timeout_is_retryable() {
        let err = ClaudeError::Timeout("Request timed out".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_transport_is_retryable() {
        let err = ClaudeError::Transport("Connection reset".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_auth_error_not_retryable() {
        let err = ClaudeError::Credential("Invalid API key".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_api_request_not_retryable() {
        let err = ClaudeError::ApiRequest("HTTP 400".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_sanitize_error_removes_api_key() {
        let msg = "Error with key sk-ant-api03-abcdefg in request";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("sk-ant-"));
        assert!(sanitized.contains("[REDACTED]"));
    }
}
