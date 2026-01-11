//! HTTP transport abstraction for Claude API
//!
//! Provides both mock and real HTTP transports for testing and production use.

use crate::error::ClaudeError;
use async_trait::async_trait;
use std::sync::Mutex;

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
    response_fn: Option<Box<dyn Fn(&str) -> Result<String, ClaudeError> + Send + Sync>>,
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
#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, ClaudeError> {
        let mut request = self.client.post(url).body(body.to_string());

        for (key, value) in headers {
            request = request.header(*key, *value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ClaudeError::ApiRequest(e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ClaudeError::ResponseParsing(e.to_string()))?;

        if !status.is_success() {
            return Err(ClaudeError::ApiRequest(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        Ok(text)
    }
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
        let transport = MockTransport::with_response(|body| {
            Ok(format!("echo: {}", body))
        });
        let result = transport.post_json("http://test", &[], "hello").await;
        assert_eq!(result.unwrap(), "echo: hello");
    }
}
