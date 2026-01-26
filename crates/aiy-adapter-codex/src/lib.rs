//! # aiy-adapter-codex
//!
//! Codex (OpenAI) adapter for the All-in-Yum multi-agent consensus pipeline.
//!
//! This crate provides a secure integration with OpenAI's GPT API, implementing
//! the `AgentAdapter` trait and enforcing prompt injection defenses.
//!
//! ## Features
//!
//! - `http` - Enable real HTTP transport using reqwest (production use)
//!
//! ## Usage
//!
//! ### With Mock Transport (default, for testing)
//! ```rust,ignore
//! use aiy_adapter_codex::{CodexClient, client::MockTransport};
//!
//! let transport = Arc::new(MockTransport::with_canned_response(response));
//! let client = CodexClient::new_with_mock(credential_manager, transport);
//! ```
//!
//! ### With Real HTTP Transport (requires `http` feature)
//! ```rust,ignore
//! use aiy_adapter_codex::CodexClient;
//!
//! let client = CodexClient::new_with_http(credential_manager)?;
//! ```

pub mod client;
pub mod error;
pub mod models;
pub mod transport;
pub mod types;

pub use client::CodexClient;
pub use error::CodexError;
pub use models::CodexModel;
pub use transport::{HttpTransport, MockTransport};
pub use types::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage};

#[cfg(feature = "http")]
pub use transport::ReqwestTransport;

use aiy_adapters::{AdapterError, AgentAdapter, AgentReview};
use async_trait::async_trait;

/// Codex adapter implementing the AgentAdapter trait
pub struct CodexAdapter {
    client: CodexClient,
}

impl CodexAdapter {
    /// Create a new CodexAdapter with a pre-configured client
    pub fn new(client: CodexClient) -> Self {
        Self { client }
    }

    /// Generate text from a prompt (inherent async method)
    pub async fn generate_text(&self, prompt: &str) -> Result<String, CodexError> {
        self.client.generate_text(prompt).await
    }

    /// Review an artifact with prompt injection defenses
    ///
    /// This method:
    /// 1. Sanitizes the artifact content
    /// 2. Builds a secure review prompt
    /// 3. Validates the response
    /// 4. Enforces schema compliance
    pub async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, CodexError> {
        self.client.review_artifact(artifact).await
    }
}

#[async_trait]
impl AgentAdapter for CodexAdapter {
    fn id(&self) -> &str {
        "codex"
    }

    fn display_name(&self) -> &str {
        "Codex (OpenAI)"
    }

    async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, AdapterError> {
        self.client
            .review_artifact(artifact)
            .await
            .map_err(|e| e.to_adapter_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiy_core::security::{CredentialBackend, CredentialManager};
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

    #[tokio::test]
    async fn test_adapter_id() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("openai", "test-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = CodexClient::new_with_mock(manager, mock_transport);
        let adapter = CodexAdapter::new(client);

        assert_eq!(adapter.id(), "codex");
    }

    #[tokio::test]
    async fn test_adapter_display_name() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("openai", "test-key").unwrap();
        }

        let mock_transport = Arc::new(MockTransport::new());
        let client = CodexClient::new_with_mock(manager, mock_transport);
        let adapter = CodexAdapter::new(client);

        assert_eq!(adapter.display_name(), "Codex (OpenAI)");
    }
}
