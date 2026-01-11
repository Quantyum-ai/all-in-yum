//! Codex API client with pluggable transport and credential integration

use crate::error::CodexError;
use crate::models::CodexModel;
use crate::transport::HttpTransport;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageRole};
use aiy_adapters::AgentReview;
use aiy_core::security::CredentialManager;
use aiy_core::security::sanitization::{
    build_secure_review_prompt, sanitize_artifact_content, validate_review_response,
    validate_review_schema,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

// Re-export MockTransport for backwards compatibility
pub use crate::transport::MockTransport;

/// Default OpenAI API base URL
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// Default timeout in milliseconds
pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Codex API client
pub struct CodexClient {
    /// Base URL for API
    base_url: String,
    /// Model to use
    model: CodexModel,
    /// Timeout in milliseconds
    timeout_ms: u64,
    /// HTTP transport (mock or real)
    transport: Arc<dyn HttpTransport>,
    /// Credential manager for API key retrieval
    credential_manager: Arc<Mutex<CredentialManager>>,
}

impl CodexClient {
    /// Create a new Codex client with mock transport (Stage A)
    pub fn new_with_mock(
        credential_manager: Arc<Mutex<CredentialManager>>,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: CodexModel::default(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            transport,
            credential_manager,
        }
    }

    /// Create a new Codex client with real HTTP transport (Stage B)
    ///
    /// This method is only available when the `http` feature is enabled.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built
    #[cfg(feature = "http")]
    pub fn new_with_http(
        credential_manager: Arc<Mutex<CredentialManager>>,
    ) -> Result<Self, CodexError> {
        use crate::transport::ReqwestTransport;
        use std::time::Duration;

        let timeout = Duration::from_millis(DEFAULT_TIMEOUT_MS);
        let transport = ReqwestTransport::with_timeout(timeout)?;

        Ok(Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: CodexModel::default(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            transport: Arc::new(transport),
            credential_manager,
        })
    }

    /// Create a new Codex client with real HTTP transport and custom timeout (Stage B)
    ///
    /// This method is only available when the `http` feature is enabled.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built
    #[cfg(feature = "http")]
    pub fn new_with_http_timeout(
        credential_manager: Arc<Mutex<CredentialManager>>,
        timeout: std::time::Duration,
    ) -> Result<Self, CodexError> {
        use crate::transport::ReqwestTransport;

        let transport = ReqwestTransport::with_timeout(timeout)?;
        let timeout_ms = timeout.as_millis() as u64;

        Ok(Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: CodexModel::default(),
            timeout_ms,
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
    pub fn with_model(mut self, model: CodexModel) -> Self {
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
    /// Tries provider names in order: "openai", "codex"
    async fn get_api_key(&self) -> Result<String, CodexError> {
        let manager = self.credential_manager.lock().await;

        // Try "openai" first
        if let Ok(key) = manager.get_key("openai") {
            return Ok(key);
        }

        // Fallback to "codex"
        if let Ok(key) = manager.get_key("codex") {
            return Ok(key);
        }

        Err(CodexError::Credential(
            "No API key found for providers 'openai' or 'codex'".to_string(),
        ))
    }

    /// Send a chat completion request
    async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, CodexError> {
        let api_key = self.get_api_key().await?;
        let url = format!("{}/chat/completions", self.base_url);

        let body = serde_json::to_string(request)?;
        let auth_header = format!("Bearer {}", api_key);
        let headers = vec![
            ("Content-Type", "application/json"),
            ("Authorization", auth_header.as_str()),
        ];

        let response_json = self.transport.post_json(&url, &headers, &body).await?;

        serde_json::from_str::<ChatCompletionResponse>(&response_json)
            .map_err(|e| CodexError::ResponseParsing(e.to_string()))
    }

    /// Generate text from a simple prompt
    pub async fn generate_text(&self, prompt: &str) -> Result<String, CodexError> {
        let request = ChatCompletionRequest::new(
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
            .ok_or_else(|| CodexError::ResponseParsing("No content in response".to_string()))
    }

    /// Review an artifact with full security defenses
    pub async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, CodexError> {
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
            .map_err(|e| CodexError::ResponseParsing(e.to_string()))?;

        // Step 6: Validate schema
        validate_review_schema(&response_json)?;

        // Step 7: Parse into AgentReview
        serde_json::from_value(response_json).map_err(|e| CodexError::ResponseParsing(e.to_string()))
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
    async fn test_credential_retrieval_openai() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under "openai"
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("openai", "test-api-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::with_canned_response(
            r#"{"id":"test","object":"chat.completion","created":1234567890,"model":"gpt-4o","choices":[{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}]}"#.to_string()
        ));

        let client = CodexClient::new_with_mock(manager.clone(), mock_transport);
        let key = client.get_api_key().await.unwrap();
        assert_eq!(key, "test-api-key");
    }

    #[tokio::test]
    async fn test_credential_retrieval_codex_fallback() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under "codex" (fallback)
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("codex", "fallback-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = CodexClient::new_with_mock(manager.clone(), mock_transport);
        let key = client.get_api_key().await.unwrap();
        assert_eq!(key, "fallback-key");
    }

    #[tokio::test]
    async fn test_generate_text_with_mock() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("openai", "test-key").unwrap();
        }

        let mock_response = r#"{
            "id": "test-id",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4o",
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
        let client = CodexClient::new_with_mock(manager, mock_transport);

        let response = client.generate_text("Hello").await.unwrap();
        assert_eq!(response, "Test response");
    }

    #[tokio::test]
    async fn test_client_builder_pattern() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        let mock_transport = Arc::new(MockTransport::new());
        let client = CodexClient::new_with_mock(manager, mock_transport)
            .with_model(CodexModel::Gpt4oMini)
            .with_base_url("https://custom.api.test".to_string())
            .with_timeout_ms(60_000);

        // The client is created successfully with custom settings
        assert!(client.base_url == "https://custom.api.test");
    }

    #[tokio::test]
    async fn test_no_credentials_error() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        // Don't store any keys

        let mock_transport = Arc::new(MockTransport::new());
        let client = CodexClient::new_with_mock(manager, mock_transport);

        let result = client.get_api_key().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No API key found"));
    }
}
