//! HTTP transport abstraction for pluggable HTTP clients
//!
//! This module provides:
//! - `HttpTransport` trait for abstracted HTTP POST
//! - `MockTransport` for testing (Stage A)
//! - `ReqwestTransport` for production HTTP (Stage B, feature-gated)

use crate::error::GrokError;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// HTTP transport trait for making API requests
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Send a POST request with JSON body
    async fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<String, GrokError>;
}

/// Mock transport for testing (Stage A)
///
/// Can be configured with canned responses or a response generator function.
pub struct MockTransport {
    responses: Vec<String>,
    current_index: AtomicUsize,
}

impl MockTransport {
    /// Create an empty mock transport (returns error on any request)
    pub fn new() -> Self {
        Self {
            responses: vec![],
            current_index: AtomicUsize::new(0),
        }
    }

    /// Create a mock transport with a single canned response
    pub fn with_canned_response(response: String) -> Self {
        Self {
            responses: vec![response],
            current_index: AtomicUsize::new(0),
        }
    }

    /// Create a mock transport with multiple responses (cycled)
    pub fn with_responses(responses: Vec<String>) -> Self {
        Self {
            responses,
            current_index: AtomicUsize::new(0),
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
    async fn post_json(&self, _url: &str, _headers: &[(&str, &str)], _body: &str) -> Result<String, GrokError> {
        if self.responses.is_empty() {
            return Err(GrokError::Transport("No mock responses configured".to_string()));
        }
        let index = self.current_index.fetch_add(1, Ordering::SeqCst);
        let response = &self.responses[index % self.responses.len()];
        Ok(response.clone())
    }
}

/// Real HTTP transport using reqwest (Stage B)
///
/// Only available when the `http` feature is enabled.
#[cfg(feature = "http")]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

#[cfg(feature = "http")]
impl ReqwestTransport {
    /// Create a new ReqwestTransport with the specified timeout
    pub fn with_timeout(timeout: Duration) -> Result<Self, GrokError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| GrokError::Transport(sanitize_error_message(&e.to_string())))?;
        Ok(Self { client })
    }

    /// Create with default timeout (30s)
    pub fn new() -> Result<Self, GrokError> {
        Self::with_timeout(Duration::from_secs(30))
    }
}

#[cfg(feature = "http")]
#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<String, GrokError> {
        let mut request = self.client.post(url)
            .body(body.to_string())
            .header("Content-Type", "application/json");

        for (key, value) in headers {
            request = request.header(*key, *value);
        }

        // Retry logic with exponential backoff
        let mut last_error = None;
        let delays = [Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)];

        for (attempt, delay) in delays.iter().enumerate() {
            let result = request.try_clone()
                .ok_or_else(|| GrokError::Transport("Failed to clone request".to_string()))?
                .send()
                .await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    // Retry on 5xx or 429
                    if status.is_server_error() || status.as_u16() == 429 {
                        if attempt < delays.len() - 1 {
                            tokio::time::sleep(*delay).await;
                            last_error = Some(GrokError::Transport(format!("Server returned {}", status)));
                            continue;
                        }
                    }

                    if !status.is_success() {
                        return Err(GrokError::ApiRequest(format!("HTTP {}", status)));
                    }

                    return response.text().await
                        .map_err(|e| GrokError::Transport(sanitize_error_message(&e.to_string())));
                }
                Err(e) => {
                    last_error = Some(GrokError::Transport(sanitize_error_message(&e.to_string())));
                    if attempt < delays.len() - 1 {
                        tokio::time::sleep(*delay).await;
                        continue;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| GrokError::Transport("Unknown error".to_string())))
    }
}

/// Sanitize error messages to prevent API key leakage
fn sanitize_error_message(msg: &str) -> String {
    // Remove anything that looks like an API key
    let sanitized = regex::Regex::new(r"(xai-[a-zA-Z0-9]{20,}|Bearer [a-zA-Z0-9\-_]+)")
        .unwrap()
        .replace_all(msg, "[REDACTED]");
    sanitized.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_transport_single_response() {
        let transport = MockTransport::with_canned_response("test response".to_string());
        let result = transport.post_json("http://test", &[], "{}").await.unwrap();
        assert_eq!(result, "test response");
    }

    #[tokio::test]
    async fn test_mock_transport_multiple_responses() {
        let transport = MockTransport::with_responses(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(transport.post_json("", &[], "").await.unwrap(), "a");
        assert_eq!(transport.post_json("", &[], "").await.unwrap(), "b");
        assert_eq!(transport.post_json("", &[], "").await.unwrap(), "a"); // cycles
    }

    #[test]
    fn test_sanitize_error_removes_api_key() {
        let msg = "Error with key xai-abcdefghij1234567890xyz in request";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("xai-"));
        assert!(sanitized.contains("[REDACTED]"));
    }
}
