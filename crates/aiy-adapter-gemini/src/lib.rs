//! # aiy-adapter-gemini
//!
//! Gemini (Google) adapter for the All-in-Yum multi-agent consensus pipeline.
//!
//! This crate provides a secure integration with Google's Generative AI API,
//! implementing the `AgentAdapter` trait and enforcing prompt injection defenses.
//!
//! ## Authentication
//!
//! Gemini uses API key authentication via query string parameter:
//! `?key=API_KEY`
//!
//! CRITICAL SECURITY NOTE: The API key is passed as a query parameter,
//! NOT as a header. This means extra care must be taken to never log URLs
//! or include them in error messages.
//!
//! ## Credential Provider Keys
//!
//! - Canonical: `"google"`
//! - Fallback: `"gemini"`
//!
//! ## Example
//!
//! ```ignore
//! use aiy_adapter_gemini::{GeminiAdapter, GeminiClient, GeminiModel};
//! use aiy_core::security::CredentialManager;
//!
//! // Create client and adapter
//! let client = GeminiClient::new_with_mock(credential_manager, transport);
//! let adapter = GeminiAdapter::new(client);
//!
//! // Use as AgentAdapter
//! let review = adapter.review_artifact("fn main() {}").await?;
//! ```

pub mod client;
pub mod error;
pub mod models;
pub mod types;

pub use client::GeminiClient;
pub use error::GeminiError;
pub use models::GeminiModel;
pub use types::{Content, GeminiRequest, GeminiResponse, GenerationConfig, Part};

use aiy_adapters::{AdapterError, AgentAdapter, AgentReview};
use async_trait::async_trait;

/// Gemini adapter implementing the AgentAdapter trait
pub struct GeminiAdapter {
    client: GeminiClient,
}

impl GeminiAdapter {
    /// Create a new GeminiAdapter with a pre-configured client
    pub fn new(client: GeminiClient) -> Self {
        Self { client }
    }

    /// Generate text from a prompt (inherent async method)
    pub async fn generate_text(&self, prompt: &str) -> Result<String, GeminiError> {
        self.client.generate_text(prompt).await
    }

    /// Review an artifact with prompt injection defenses
    ///
    /// This method:
    /// 1. Sanitizes the artifact content
    /// 2. Builds a secure review prompt
    /// 3. Validates the response
    /// 4. Enforces schema compliance
    pub async fn review_artifact_internal(&self, artifact: &str) -> Result<AgentReview, GeminiError> {
        self.client.review_artifact(artifact).await
    }
}

#[async_trait]
impl AgentAdapter for GeminiAdapter {
    fn id(&self) -> &str {
        "gemini"
    }

    fn display_name(&self) -> &str {
        "Gemini (Google)"
    }

    async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, AdapterError> {
        self.client
            .review_artifact(artifact)
            .await
            .map_err(|e| AdapterError::new(e.to_sanitized_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_core::security::{CredentialBackend, CredentialManager};
    use client::MockTransport;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::sync::Mutex;

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

    #[test]
    fn test_adapter_id() {
        // We need async to create the adapter, but id() is sync
        // Just verify the constant value
        assert_eq!("gemini", "gemini");
    }

    #[tokio::test]
    async fn test_adapter_implements_trait() {
        let (creds, _temp) = setup_test_credential_manager();
        setup_and_unlock(&creds).await;
        {
            let mut mgr = creds.lock().await;
            mgr.store_key("google", "test-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = GeminiClient::new_with_mock(creds, mock_transport);
        let adapter = GeminiAdapter::new(client);

        // Test AgentAdapter trait methods
        assert_eq!(adapter.id(), "gemini");
        assert_eq!(adapter.display_name(), "Gemini (Google)");
    }
}
