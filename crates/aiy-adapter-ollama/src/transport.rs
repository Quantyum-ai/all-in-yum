//! HTTP transport abstraction for Ollama API
//!
//! Provides both mock and real HTTP transports for testing and production use.

use crate::error::OllamaError;
use async_trait::async_trait;
use std::sync::Mutex;
#[cfg(feature = "http")]
use std::time::Duration;

/// Type alias for mock response generator functions
type MockResponseFn = Box<dyn Fn(&str) -> Result<String, OllamaError> + Send + Sync>;

/// HTTP transport trait for making API requests to Ollama
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Send a POST request with JSON body
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, OllamaError>;

    /// Send a GET request
    async fn get(&self, url: &str) -> Result<String, OllamaError>;
}

/// Mock transport for testing
pub struct MockTransport {
    /// Canned response to return for POST
    post_response: Mutex<Option<String>>,
    /// Canned response to return for GET
    get_response: Mutex<Option<String>>,
    /// Custom response function for POST
    response_fn: Option<MockResponseFn>,
}

impl MockTransport {
    /// Create a new mock transport with no canned response
    pub fn new() -> Self {
        Self {
            post_response: Mutex::new(None),
            get_response: Mutex::new(None),
            response_fn: None,
        }
    }

    /// Create a mock transport with a canned POST response
    pub fn with_canned_response(response: String) -> Self {
        Self {
            post_response: Mutex::new(Some(response)),
            get_response: Mutex::new(None),
            response_fn: None,
        }
    }

    /// Create a mock transport with canned responses for both POST and GET
    pub fn with_responses(post: String, get: String) -> Self {
        Self {
            post_response: Mutex::new(Some(post)),
            get_response: Mutex::new(Some(get)),
            response_fn: None,
        }
    }

    /// Create a mock transport with a custom response function
    pub fn with_response<F>(f: F) -> Self
    where
        F: Fn(&str) -> Result<String, OllamaError> + Send + Sync + 'static,
    {
        Self {
            post_response: Mutex::new(None),
            get_response: Mutex::new(None),
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
    ) -> Result<String, OllamaError> {
        // If we have a custom response function, use it
        if let Some(ref f) = self.response_fn {
            return f(body);
        }

        // Otherwise, return the canned response
        let response = self.post_response.lock().unwrap();
        response
            .clone()
            .ok_or_else(|| OllamaError::Transport("No mock response configured".to_string()))
    }

    async fn get(&self, _url: &str) -> Result<String, OllamaError> {
        let response = self.get_response.lock().unwrap();
        response
            .clone()
            .ok_or_else(|| OllamaError::Transport("No mock GET response configured".to_string()))
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
    pub fn new() -> Result<Self, OllamaError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| OllamaError::Transport(e.to_string()))?;
        Ok(Self { client })
    }

    /// Create a new reqwest transport with custom timeout
    pub fn with_timeout(timeout: Duration) -> Result<Self, OllamaError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| OllamaError::Transport(e.to_string()))?;
        Ok(Self { client })
    }
}

#[cfg(feature = "http")]
impl ReqwestTransport {
    /// Execute a single POST request without retries
    async fn execute_post(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, OllamaError> {
        let mut req = self.client.post(url).body(body.to_string());
        for (key, value) in headers {
            req = req.header(*key, *value);
        }

        let response = req
            .send()
            .await
            .map_err(|e| OllamaError::Transport(e.to_string()))?;

        let status = response.status();

        if status.as_u16() == 503 {
            return Err(OllamaError::ResourceExhausted(
                "Ollama server unavailable".to_string(),
            ));
        }
        if status.is_server_error() {
            return Err(OllamaError::Transport(format!("Server error: {}", status)));
        }
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            if error_body.contains("model") && error_body.contains("not found") {
                return Err(OllamaError::ModelNotAvailable(
                    "Model not found on server".to_string(),
                ));
            }
            return Err(OllamaError::ApiRequest(format!("HTTP {}", status)));
        }

        response
            .text()
            .await
            .map_err(|e| OllamaError::Transport(e.to_string()))
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
    ) -> Result<String, OllamaError> {
        // Retry logic with exponential backoff for transient errors
        let delays = [
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
        ];
        let max_retries = delays.len();

        for attempt in 0..=max_retries {
            match self.execute_post(url, headers, body).await {
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

    async fn get(&self, url: &str) -> Result<String, OllamaError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| OllamaError::Connection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OllamaError::ApiRequest(format!(
                "HTTP {}",
                response.status()
            )));
        }

        response
            .text()
            .await
            .map_err(|e| OllamaError::Transport(e.to_string()))
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
        let transport = MockTransport::with_response(|body| Ok(format!("echo: {}", body)));
        let result = transport.post_json("http://test", &[], "hello").await;
        assert_eq!(result.unwrap(), "echo: hello");
    }

    #[tokio::test]
    async fn test_mock_transport_get() {
        let transport = MockTransport::with_responses(
            "post response".to_string(),
            r#"{"models":[]}"#.to_string(),
        );
        let result = transport.get("http://test/api/tags").await;
        assert_eq!(result.unwrap(), r#"{"models":[]}"#);
    }
}
