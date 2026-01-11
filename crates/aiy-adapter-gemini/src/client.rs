//! Gemini API client with pluggable transport and credential integration

use crate::error::GeminiError;
use crate::models::GeminiModel;
use crate::types::{GeminiRequest, GeminiResponse, GenerationConfig};
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

/// Default Google Generative AI API base URL
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default timeout in milliseconds
pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// Trait for HTTP transport (allows mocking in Stage A)
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Send a POST request with JSON body and return JSON response
    /// NOTE: For Gemini, the URL will include the API key as a query parameter.
    /// Implementations must ensure the key is never logged.
    async fn post_json(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, GeminiError>;
}

/// Type alias for mock response function
type ResponseFn = Box<dyn Fn(&str) -> Result<String, GeminiError> + Send + Sync>;

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
        F: Fn(&str) -> Result<String, GeminiError> + Send + Sync + 'static,
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
    ) -> Result<String, GeminiError> {
        if let Some(ref response_fn) = self.response_fn {
            response_fn(body)
        } else {
            Err(GeminiError::Transport(
                "MockTransport: No response configured".to_string(),
            ))
        }
    }
}

/// Gemini API client
pub struct GeminiClient {
    /// Base URL for API
    base_url: String,
    /// Model to use
    model: GeminiModel,
    /// Timeout in milliseconds
    timeout_ms: u64,
    /// HTTP transport (mock or real)
    transport: Arc<dyn HttpTransport>,
    /// Credential manager for API key retrieval
    credential_manager: Arc<Mutex<CredentialManager>>,
}

impl GeminiClient {
    /// Create a new Gemini client with mock transport (Stage A)
    pub fn new_with_mock(
        credential_manager: Arc<Mutex<CredentialManager>>,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: GeminiModel::default(),
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
    pub fn with_model(mut self, model: GeminiModel) -> Self {
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
    /// Tries provider names in order: "google", "gemini"
    async fn get_api_key(&self) -> Result<String, GeminiError> {
        let manager = self.credential_manager.lock().await;

        // Try "google" first (canonical)
        if let Ok(key) = manager.get_key("google") {
            return Ok(key);
        }

        // Fallback to "gemini"
        if let Ok(key) = manager.get_key("gemini") {
            return Ok(key);
        }

        Err(GeminiError::Credential(
            "No API key found for providers 'google' or 'gemini'".to_string(),
        ))
    }

    /// Build the API endpoint URL with the API key as a query parameter
    /// SECURITY: This URL should never be logged or included in error messages!
    fn build_endpoint_url(&self, api_key: &str) -> String {
        format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url,
            self.model.as_str(),
            api_key
        )
    }

    /// Send a generateContent request
    async fn generate_content(&self, request: &GeminiRequest) -> Result<GeminiResponse, GeminiError> {
        let api_key = self.get_api_key().await?;
        let url = self.build_endpoint_url(&api_key);

        let body = serde_json::to_string(request)?;
        let headers = vec![
            ("Content-Type", "application/json"),
        ];

        let response_json = self.transport.post_json(&url, &headers, &body).await?;

        serde_json::from_str::<GeminiResponse>(&response_json)
            .map_err(|e| GeminiError::ResponseParsing(e.to_string()))
    }

    /// Generate text from a simple prompt
    pub async fn generate_text(&self, prompt: &str) -> Result<String, GeminiError> {
        let request = GeminiRequest::new(prompt);

        let response = self.generate_content(&request).await?;

        response
            .text()
            .ok_or_else(|| GeminiError::ResponseParsing("No content in response".to_string()))
    }

    /// Generate text with custom generation config
    pub async fn generate_text_with_config(
        &self,
        prompt: &str,
        config: GenerationConfig,
    ) -> Result<String, GeminiError> {
        let request = GeminiRequest::new(prompt).with_generation_config(config);

        let response = self.generate_content(&request).await?;

        response
            .text()
            .ok_or_else(|| GeminiError::ResponseParsing("No content in response".to_string()))
    }

    /// Review an artifact with full security defenses
    pub async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, GeminiError> {
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
            .map_err(|e| GeminiError::ResponseParsing(e.to_string()))?;

        // Step 6: Validate schema
        validate_review_schema(&response_json)?;

        // Step 7: Parse into AgentReview
        serde_json::from_value(response_json).map_err(|e| GeminiError::ResponseParsing(e.to_string()))
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
    async fn test_credential_retrieval_google() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under "google"
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("google", "test-api-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::with_canned_response(
            r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}],"role":"model"},"finishReason":"STOP"}]}"#.to_string()
        ));

        let client = GeminiClient::new_with_mock(manager.clone(), mock_transport);
        let key = client.get_api_key().await.unwrap();
        assert_eq!(key, "test-api-key");
    }

    #[tokio::test]
    async fn test_credential_retrieval_gemini_fallback() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under "gemini" (fallback)
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("gemini", "fallback-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = GeminiClient::new_with_mock(manager.clone(), mock_transport);
        let key = client.get_api_key().await.unwrap();
        assert_eq!(key, "fallback-key");
    }

    #[tokio::test]
    async fn test_generate_text_with_mock() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("google", "test-key").unwrap();
        }

        let mock_response = r#"{
            "candidates": [{
                "content": {
                    "parts": [{"text": "Test response from Gemini"}],
                    "role": "model"
                },
                "finishReason": "STOP"
            }]
        }"#;

        let mock_transport = Arc::new(MockTransport::with_canned_response(mock_response.to_string()));
        let client = GeminiClient::new_with_mock(manager, mock_transport);

        let response = client.generate_text("Hello").await.unwrap();
        assert_eq!(response, "Test response from Gemini");
    }

    #[tokio::test]
    async fn test_endpoint_url_contains_key_as_query_param() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("google", "secret-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = GeminiClient::new_with_mock(manager, mock_transport);

        // Build URL and verify it contains key as query param (not header)
        let url = client.build_endpoint_url("test-key-123");
        assert!(url.contains("?key=test-key-123"));
        assert!(url.contains("generateContent"));
        assert!(url.contains(GeminiModel::default().as_str()));
    }

    #[tokio::test]
    async fn test_model_configuration() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("google", "key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = GeminiClient::new_with_mock(manager, mock_transport)
            .with_model(GeminiModel::Gemini15Flash)
            .with_base_url("https://custom.api.test".to_string())
            .with_timeout_ms(60_000);

        // Verify model is in the URL
        let url = client.build_endpoint_url("test");
        assert!(url.contains("gemini-1.5-flash-latest"));
        assert!(url.contains("custom.api.test"));
    }
}
