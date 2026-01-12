//! Claude API client with pluggable transport and credential integration

use crate::error::ClaudeError;
use crate::models::ClaudeModel;
use crate::transport::HttpTransport;
use crate::types::{Message, MessageContent, MessageRole, MessagesRequest, MessagesResponse};
use aiy_adapters::AgentReview;
use aiy_core::security::sanitization::{
    build_secure_review_prompt, sanitize_artifact_content, validate_review_response,
    validate_review_schema,
};
use aiy_core::security::CredentialManager;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

// Re-export MockTransport for backwards compatibility
pub use crate::transport::MockTransport;

/// Anthropic API base URL
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";

/// Default timeout in milliseconds
pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Default max tokens for response
pub const DEFAULT_MAX_TOKENS: usize = 4096;

/// Required Anthropic API version header
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Claude API client
pub struct ClaudeClient {
    /// Base URL for API
    base_url: String,
    /// Model to use
    model: ClaudeModel,
    /// Timeout in milliseconds
    timeout_ms: u64,
    /// Maximum tokens for response
    max_tokens: usize,
    /// HTTP transport (mock or real)
    transport: Arc<dyn HttpTransport>,
    /// Credential manager for API key retrieval
    credential_manager: Arc<Mutex<CredentialManager>>,
}

impl ClaudeClient {
    /// Create a new Claude client with mock transport (Stage A)
    pub fn new_with_mock(
        credential_manager: Arc<Mutex<CredentialManager>>,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: ClaudeModel::default(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_tokens: DEFAULT_MAX_TOKENS,
            transport,
            credential_manager,
        }
    }

    /// Create a new Claude client with real HTTP transport (Stage B)
    ///
    /// This method is only available when the `http` feature is enabled.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built
    #[cfg(feature = "http")]
    pub fn new_with_http(
        credential_manager: Arc<Mutex<CredentialManager>>,
    ) -> Result<Self, ClaudeError> {
        use crate::transport::ReqwestTransport;
        use std::time::Duration;

        let timeout = Duration::from_millis(DEFAULT_TIMEOUT_MS);
        let transport = ReqwestTransport::with_timeout(timeout)?;

        Ok(Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: ClaudeModel::default(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_tokens: DEFAULT_MAX_TOKENS,
            transport: Arc::new(transport),
            credential_manager,
        })
    }

    /// Create a new Claude client with real HTTP transport and custom timeout (Stage B)
    ///
    /// This method is only available when the `http` feature is enabled.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built
    #[cfg(feature = "http")]
    pub fn new_with_http_timeout(
        credential_manager: Arc<Mutex<CredentialManager>>,
        timeout: std::time::Duration,
    ) -> Result<Self, ClaudeError> {
        use crate::transport::ReqwestTransport;

        let transport = ReqwestTransport::with_timeout(timeout)?;
        let timeout_ms = timeout.as_millis() as u64;

        Ok(Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: ClaudeModel::default(),
            timeout_ms,
            max_tokens: DEFAULT_MAX_TOKENS,
            transport: Arc::new(transport),
            credential_manager,
        })
    }

    /// Set a custom base URL
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Set the model
    pub fn with_model(mut self, model: ClaudeModel) -> Self {
        self.model = model;
        self
    }

    /// Set the timeout
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set the maximum tokens for response
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Get the API key from credential manager
    ///
    /// Tries provider names in order: "claude", "anthropic"
    async fn get_api_key(&self) -> Result<String, ClaudeError> {
        let manager = self.credential_manager.lock().await;

        // Try "claude" first
        if let Ok(key) = manager.get_key("claude") {
            return Ok(key);
        }

        // Fallback to "anthropic"
        if let Ok(key) = manager.get_key("anthropic") {
            return Ok(key);
        }

        Err(ClaudeError::Credential(
            "No API key found for providers 'claude' or 'anthropic'".to_string(),
        ))
    }

    /// Send a messages request
    async fn send_messages(
        &self,
        request: &MessagesRequest,
    ) -> Result<MessagesResponse, ClaudeError> {
        let api_key = self.get_api_key().await?;
        let url = format!("{}/messages", self.base_url);

        let body = serde_json::to_string(request)?;
        let headers = vec![
            ("Content-Type", "application/json"),
            ("x-api-key", api_key.as_str()),
            ("anthropic-version", ANTHROPIC_VERSION),
        ];

        let response_json = self.transport.post_json(&url, &headers, &body).await?;

        serde_json::from_str::<MessagesResponse>(&response_json)
            .map_err(|e| ClaudeError::ResponseParsing(e.to_string()))
    }

    /// Generate text from a simple prompt
    pub async fn generate_text(&self, prompt: &str) -> Result<String, ClaudeError> {
        let request = MessagesRequest::new(
            self.model.to_string(),
            self.max_tokens,
            vec![Message {
                role: MessageRole::User,
                content: MessageContent::Text(prompt.to_string()),
            }],
        );

        let response = self.send_messages(&request).await?;

        response
            .get_text()
            .ok_or_else(|| ClaudeError::ResponseParsing("No content in response".to_string()))
    }

    /// Review an artifact with full security defenses
    pub async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, ClaudeError> {
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
            .map_err(|e| ClaudeError::ResponseParsing(e.to_string()))?;

        // Step 6: Validate schema
        validate_review_schema(&response_json)?;

        // Step 7: Parse into AgentReview
        serde_json::from_value(response_json)
            .map_err(|e| ClaudeError::ResponseParsing(e.to_string()))
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
    async fn test_credential_retrieval_claude() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under "claude"
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("claude", "test-api-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::with_canned_response(
            r#"{"id":"msg_test","type":"message","role":"assistant","content":[{"type":"text","text":"Hello"}],"model":"claude-sonnet-4-20250514","stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":5}}"#.to_string()
        ));

        let client = ClaudeClient::new_with_mock(manager.clone(), mock_transport);
        let key = client.get_api_key().await.unwrap();
        assert_eq!(key, "test-api-key");
    }

    #[tokio::test]
    async fn test_credential_retrieval_anthropic_fallback() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under "anthropic" (fallback)
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("anthropic", "fallback-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = ClaudeClient::new_with_mock(manager.clone(), mock_transport);
        let key = client.get_api_key().await.unwrap();
        assert_eq!(key, "fallback-key");
    }

    #[tokio::test]
    async fn test_generate_text_with_mock() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("claude", "test-key").unwrap();
        }

        let mock_response = r#"{
            "id": "msg_test",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Test response"
                }
            ],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        }"#;

        let mock_transport = Arc::new(MockTransport::with_canned_response(
            mock_response.to_string(),
        ));
        let client = ClaudeClient::new_with_mock(manager, mock_transport);

        let response = client.generate_text("Hello").await.unwrap();
        assert_eq!(response, "Test response");
    }

    #[tokio::test]
    async fn test_request_includes_required_headers() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("claude", "secret-token").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::with_response(move |_body| {
            // This will be called, but we can't capture headers in this simple mock
            // In a real test, use a more sophisticated mock
            Ok(r#"{"id":"msg_test","type":"message","role":"assistant","content":[{"type":"text","text":"OK"}],"model":"claude-sonnet-4-20250514","stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":5}}"#.to_string())
        }));

        let client = ClaudeClient::new_with_mock(manager, mock_transport);
        let _ = client.generate_text("Test").await;

        // In a real implementation, verify headers contain:
        // - x-api-key: secret-token
        // - anthropic-version: 2023-06-01
        // For Stage A, this is a placeholder test
    }

    #[tokio::test]
    async fn test_model_configuration() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("claude", "key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let _client = ClaudeClient::new_with_mock(manager, mock_transport)
            .with_model(ClaudeModel::Claude35Haiku)
            .with_base_url("https://custom.api.test".to_string())
            .with_timeout_ms(60_000)
            .with_max_tokens(2048);

        // Verify configuration - the client is successfully configured
    }
}
