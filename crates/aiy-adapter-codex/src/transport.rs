//! HTTP transport abstraction for the Codex adapter
//!
//! Provides both mock transport for testing and real HTTP transport (feature-gated).

use crate::error::CodexError;
use async_trait::async_trait;
use std::sync::Mutex;

/// HTTP transport trait for making API requests
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Send a POST request with JSON body
    ///
    /// # Arguments
    /// * `url` - The URL to POST to
    /// * `headers` - Headers as (key, value) pairs
    /// * `body` - JSON body as a string
    ///
    /// # Returns
    /// The response body as a string, or an error
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, CodexError>;
}

/// Mock transport for testing
pub struct MockTransport {
    /// Canned response to return
    response: Mutex<Option<String>>,
    /// Response generator function
    response_fn: Option<Box<dyn Fn(&str) -> Result<String, CodexError> + Send + Sync>>,
}

impl MockTransport {
    /// Create a new mock transport that returns an error
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

    /// Create a mock transport with a response generator function
    pub fn with_response<F>(f: F) -> Self
    where
        F: Fn(&str) -> Result<String, CodexError> + Send + Sync + 'static,
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
    ) -> Result<String, CodexError> {
        if let Some(ref f) = self.response_fn {
            return f(body);
        }

        let response = self.response.lock().unwrap();
        response
            .clone()
            .ok_or_else(|| CodexError::Transport("No mock response configured".to_string()))
    }
}

/// Real HTTP transport using reqwest (feature-gated)
#[cfg(feature = "http")]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

#[cfg(feature = "http")]
impl ReqwestTransport {
    /// Create a new reqwest transport with default timeout
    pub fn new() -> Result<Self, CodexError> {
        Self::with_timeout(std::time::Duration::from_secs(120))
    }

    /// Create a new reqwest transport with custom timeout
    pub fn with_timeout(timeout: std::time::Duration) -> Result<Self, CodexError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| CodexError::Transport(e.to_string()))?;

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
    ) -> Result<String, CodexError> {
        let mut request = self.client.post(url).body(body.to_string());

        for (key, value) in headers {
            request = request.header(*key, *value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| CodexError::Transport(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(CodexError::ApiRequest(format!(
                "HTTP {} - {}",
                status, body
            )));
        }

        response
            .text()
            .await
            .map_err(|e| CodexError::Transport(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_transport_canned_response() {
        let transport = MockTransport::with_canned_response("test response".to_string());
        let result = transport
            .post_json("http://test", &[], "body")
            .await
            .unwrap();
        assert_eq!(result, "test response");
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
            Ok(format!("received: {}", body))
        });
        let result = transport
            .post_json("http://test", &[], "hello")
            .await
            .unwrap();
        assert_eq!(result, "received: hello");
    }
}
