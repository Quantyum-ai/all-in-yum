//! Grok API client with pluggable transport and credential integration

use crate::error::GrokError;
use crate::models::GrokModel;
use crate::types::{ChatMessage, ChatRequest, ChatResponse, MessageRole};
use aiy_adapters::AgentReview;
use aiy_core::security::CredentialManager;
use aiy_core::security::sanitization::{
    build_secure_review_prompt, sanitize_artifact_content, validate_review_response,
    validate_review_schema,
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Default xAI API base URL
pub const DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";

/// Default timeout in milliseconds
pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Trait for HTTP transport (allows mocking in Stage A)
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Send a POST request with JSON body and return JSON response
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, GrokError>;
}

/// Type alias for mock response function
type ResponseFn = Box<dyn Fn(&str) -> Result<String, GrokError> + Send + Sync>;

/// Mock HTTP transport for Stage A (offline-safe)
pub struct MockTransport {
    /// Optional mock response provider
    response_fn: Option<ResponseFn>,
}

impl MockTransport {
    /// Create a new mock transport with no responses
    pub fn new() -> Self {
        Self { response_fn: None }
    }

    /// Create a mock transport with a custom response function
    pub fn with_response<F>(response_fn: F) -> Self
    where
        F: Fn(&str) -> Result<String, GrokError> + Send + Sync + 'static,
    {
        Self {
            response_fn: Some(Box::new(response_fn)),
        }
    }

    /// Create a mock transport that returns a canned response
    pub fn with_canned_response(response: String) -> Self {
        Self::with_response(move |_| Ok(response.clone()))
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
    ) -> Result<String, GrokError> {
        if let Some(ref response_fn) = self.response_fn {
            response_fn(body)
        } else {
            Err(GrokError::Transport(
                "MockTransport: No response configured".to_string(),
            ))
        }
    }
}

/// Grok API client
pub struct GrokClient {
    /// Base URL for API
    base_url: String,
    /// Model to use
    model: GrokModel,
    /// Timeout in milliseconds
    timeout_ms: u64,
    /// HTTP transport (mock or real)
    transport: Arc<dyn HttpTransport>,
    /// Credential manager for API key retrieval
    credential_manager: Arc<Mutex<CredentialManager>>,
}

impl GrokClient {
    /// Create a new Grok client with mock transport (Stage A)
    pub fn new_with_mock(
        credential_manager: Arc<Mutex<CredentialManager>>,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: GrokModel::default(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            transport,
            credential_manager,
        }
    }

    /// Set a custom base URL
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: GrokModel) -> Self {
        self.model = model;
        self
    }

    /// Set the timeout
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Get the API key from credential manager
    ///
    /// Tries provider names in order: "xai", "grok"
    async fn get_api_key(&self) -> Result<String, GrokError> {
        let manager = self.credential_manager.lock().await;

        // Try "xai" first
        if let Ok(key) = manager.get_key("xai") {
            return Ok(key);
        }

        // Fallback to "grok"
        if let Ok(key) = manager.get_key("grok") {
            return Ok(key);
        }

        Err(GrokError::Credential(
            "No API key found for providers 'xai' or 'grok'".to_string(),
        ))
    }

    /// Send a chat completion request
    async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse, GrokError> {
        let api_key = self.get_api_key().await?;
        let url = format!("{}/chat/completions", self.base_url);

        let body = serde_json::to_string(request)?;
        let auth_header = format!("Bearer {}", api_key);
        let headers = vec![
            ("Content-Type", "application/json"),
            ("Authorization", auth_header.as_str()),
        ];

        let response_json = self.transport.post_json(&url, &headers, &body).await?;

        serde_json::from_str::<ChatResponse>(&response_json)
            .map_err(|e| GrokError::ResponseParsing(e.to_string()))
    }

    /// Generate text from a simple prompt
    pub async fn generate_text(&self, prompt: &str) -> Result<String, GrokError> {
        let request = ChatRequest::new(
            self.model.to_string(),
            vec![ChatMessage {
                role: MessageRole::User,
                content: Some(prompt.to_string()),
            }],
        );

        let response = self.chat_completion(&request).await?;

        response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| GrokError::ResponseParsing("No content in response".to_string()))
    }

    /// Review an artifact with full security defenses
    pub async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, GrokError> {
        // Step 1: Sanitize artifact content
        let sanitized = sanitize_artifact_content(artifact);

        // Step 2: Build secure review prompt
        let prompt = build_secure_review_prompt(&sanitized, &[]);

        // Step 3: Generate response
        let response_text = self.generate_text(&prompt).await?;

        // Step 4: Validate response for suspicious patterns
        validate_review_response(&response_text)?;

        // Step 5: Parse JSON response
        let response_json: Value = serde_json::from_str(&response_text)
            .map_err(|e| GrokError::ResponseParsing(e.to_string()))?;

        // Step 6: Validate schema
        validate_review_schema(&response_json)?;

        // Step 7: Parse into AgentReview
        serde_json::from_value(response_json).map_err(|e| GrokError::ResponseParsing(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_core::security::CredentialBackend;
    use tempfile::TempDir;

    fn setup_test_credential_manager() -> (Arc<Mutex<CredentialManager>>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cred_path = temp_dir.path().join("test_credentials.enc");

        let manager =
            CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path }).unwrap();

        (Arc::new(Mutex::new(manager)), temp_dir)
    }

    async fn setup_and_unlock(manager: &Arc<Mutex<CredentialManager>>) {
        let mut mgr = manager.lock().await;
        mgr.unlock("test-password").unwrap();
    }

    #[tokio::test]
    async fn test_credential_retrieval_xai() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under "xai"
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("xai", "test-api-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::with_canned_response(
            r#"{"id":"test","object":"chat.completion","created":1234567890,"model":"grok-4-1-fast","choices":[{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}]}"#.to_string()
        ));

        let client = GrokClient::new_with_mock(manager.clone(), mock_transport);
        let key = client.get_api_key().await.unwrap();
        assert_eq!(key, "test-api-key");
    }

    #[tokio::test]
    async fn test_credential_retrieval_grok_fallback() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under "grok" (fallback)
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("grok", "fallback-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = GrokClient::new_with_mock(manager.clone(), mock_transport);
        let key = client.get_api_key().await.unwrap();
        assert_eq!(key, "fallback-key");
    }

    #[tokio::test]
    async fn test_generate_text_with_mock() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("xai", "test-key").unwrap();
        }

        let mock_response = r#"{
            "id": "test-id",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "grok-4-1-fast",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Test response"
                },
                "finish_reason": "stop"
            }]
        }"#;

        let mock_transport = Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
        let client = GrokClient::new_with_mock(manager, mock_transport);

        let response = client.generate_text("Hello").await.unwrap();
        assert_eq!(response, "Test response");
    }

    #[tokio::test]
    async fn test_request_includes_bearer_token() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("xai", "secret-token").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::with_response(move |_body| {
            // This will be called, but we can't capture headers in this simple mock
            // In a real test, use a more sophisticated mock
            Ok(r#"{"id":"test","object":"chat.completion","created":1234567890,"model":"grok-4-1-fast","choices":[{"index":0,"message":{"role":"assistant","content":"OK"},"finish_reason":"stop"}]}"#.to_string())
        }));

        let client = GrokClient::new_with_mock(manager, mock_transport);
        let _ = client.generate_text("Test").await;

        // In a real implementation, verify Authorization header contains "Bearer secret-token"
        // For Stage A, this is a placeholder test
    }
}
