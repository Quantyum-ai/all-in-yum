//! Gemini API client with pluggable transport and credential integration

use crate::error::GeminiError;
use crate::models::GeminiModel;
use crate::transport::HttpTransport;
use crate::types::{GeminiRequest, GeminiResponse, GenerationConfig};
use aiy_adapters::AgentReview;
use aiy_core::security::sanitization::{
    build_secure_review_prompt, sanitize_artifact_content, validate_review_response,
    validate_review_schema,
};
use aiy_core::security::CredentialManager;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Default Google Generative AI API base URL
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Default timeout in milliseconds
pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;

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

    /// Create a new Gemini client with real HTTP transport
    ///
    /// This constructor is only available when the `http` feature is enabled.
    /// It creates a client that makes real API calls to Google's Generative AI API.
    ///
    /// # Arguments
    ///
    /// * `credential_manager` - Shared credential manager for API key retrieval
    ///
    /// # Returns
    ///
    /// A new `GeminiClient` configured with real HTTP transport, or an error
    /// if the transport could not be created.
    #[cfg(feature = "http")]
    pub fn new_with_http(
        credential_manager: Arc<Mutex<CredentialManager>>,
    ) -> Result<Self, GeminiError> {
        use crate::transport::ReqwestTransport;

        let transport = ReqwestTransport::new()?;

        Ok(Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: GeminiModel::default(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
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
    async fn generate_content(
        &self,
        request: &GeminiRequest,
    ) -> Result<GeminiResponse, GeminiError> {
        let api_key = self.get_api_key().await?;
        let url = self.build_endpoint_url(&api_key);

        let body = serde_json::to_string(request)?;

        // NOTE: For Gemini, API key is in the query string, not headers.
        // The transport handles setting Content-Type.
        let response_json = self.transport.post_json(&url, &body).await?;

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
        serde_json::from_value(response_json)
            .map_err(|e| GeminiError::ResponseParsing(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MockTransport;
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

        let mock_transport = Arc::new(MockTransport::with_canned_response(
            mock_response.to_string(),
        ));
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
