//! # aiy-adapter-ollama
//!
//! Ollama adapter for the All-in-Yum privacy mode local code execution.
//!
//! This crate provides a secure integration with Ollama for local LLM inference,
//! implementing the `AgentAdapter` trait for code review operations while keeping
//! all code strictly local.
//!
//! ## Features
//!
//! - `http` - Enable real HTTP transport using reqwest (production use)
//!
//! ## Usage
//!
//! ### With Mock Transport (default, for testing)
//! ```rust,ignore
//! use aiy_adapter_ollama::{OllamaClient, transport::MockTransport};
//! use std::sync::Arc;
//!
//! let transport = Arc::new(MockTransport::with_canned_response(response));
//! let client = OllamaClient::new_with_mock(transport);
//! ```
//!
//! ### With Real HTTP Transport (requires `http` feature)
//! ```rust,ignore
//! use aiy_adapter_ollama::OllamaClient;
//!
//! let client = OllamaClient::new_with_http()?;
//! ```
//!
//! ## Privacy Mode
//!
//! This adapter is designed for privacy mode where:
//! - All code stays local (never sent to cloud)
//! - Ollama runs on localhost only
//! - Used for code generation and modification tasks

pub mod client;
pub mod error;
pub mod models;
pub mod transport;
pub mod types;

pub use client::OllamaClient;
pub use error::OllamaError;
pub use models::OllamaModel;
pub use transport::{HttpTransport, MockTransport};
pub use types::{ChatRequest, ChatResponse, GenerationOptions, Message, TagsResponse};

#[cfg(feature = "http")]
pub use transport::ReqwestTransport;

use aiy_adapters::{AdapterError, AgentAdapter, AgentReview};
use async_trait::async_trait;

/// Ollama adapter implementing the AgentAdapter trait
///
/// This adapter is used in privacy mode to perform local code reviews
/// without sending code to cloud services.
pub struct OllamaAdapter {
    client: OllamaClient,
}

impl OllamaAdapter {
    /// Create a new OllamaAdapter with a pre-configured client
    pub fn new(client: OllamaClient) -> Self {
        Self { client }
    }

    /// Generate text from a prompt (inherent async method)
    pub async fn generate_text(&self, prompt: &str) -> Result<String, OllamaError> {
        self.client.generate_text(prompt).await
    }

    /// Generate text with a system prompt
    pub async fn generate_with_system(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, OllamaError> {
        self.client
            .generate_with_system(system_prompt, user_prompt)
            .await
    }

    /// Review an artifact with local model
    ///
    /// This method keeps all code local and never sends it to cloud services.
    pub async fn review_artifact_local(&self, artifact: &str) -> Result<AgentReview, OllamaError> {
        self.client.review_artifact(artifact).await
    }

    /// Perform a health check
    pub async fn health_check(&self) -> Result<Vec<String>, OllamaError> {
        self.client.health_check().await
    }

    /// Check if the configured model is available
    pub async fn is_model_available(&self) -> Result<bool, OllamaError> {
        self.client
            .is_model_available(self.client.model().as_str())
            .await
    }
}

#[async_trait]
impl AgentAdapter for OllamaAdapter {
    fn id(&self) -> &str {
        "ollama"
    }

    fn display_name(&self) -> &str {
        "Ollama (Local)"
    }

    async fn health_check(&self) -> Result<(), AdapterError> {
        self.client
            .health_check()
            .await
            .map(|_| ())
            .map_err(|e| e.to_adapter_error())
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
    use std::sync::Arc;

    fn mock_chat_response(content: &str) -> String {
        format!(
            r#"{{"model":"codellama:7b-instruct","message":{{"role":"assistant","content":"{}"}},"done":true}}"#,
            content.replace('"', r#"\""#).replace('\n', r#"\n"#)
        )
    }

    fn mock_review_json() -> String {
        r#"{"agent_id":"ollama","verdict":"pass","confidence":0.9,"issues":[],"suggestions":[],"sign_off":true,"reasoning":"Code is good"}"#.to_string()
    }

    #[tokio::test]
    async fn test_adapter_id() {
        let transport = Arc::new(MockTransport::new());
        let client = OllamaClient::new_with_mock(transport);
        let adapter = OllamaAdapter::new(client);

        assert_eq!(adapter.id(), "ollama");
        assert_eq!(adapter.display_name(), "Ollama (Local)");
    }

    #[tokio::test]
    async fn test_adapter_review_artifact() {
        let transport =
            Arc::new(MockTransport::with_canned_response(mock_chat_response(&mock_review_json())));
        let client = OllamaClient::new_with_mock(transport);
        let adapter = OllamaAdapter::new(client);

        let result: Result<AgentReview, AdapterError> =
            adapter.review_artifact("fn main() {}").await;
        assert!(result.is_ok());

        let review = result.unwrap();
        assert_eq!(review.agent_id, "ollama");
        assert!(review.sign_off);
    }

    #[tokio::test]
    async fn test_adapter_generate_text() {
        let transport = Arc::new(MockTransport::with_canned_response(mock_chat_response(
            "Hello, world!",
        )));
        let client = OllamaClient::new_with_mock(transport);
        let adapter = OllamaAdapter::new(client);

        let result = adapter.generate_text("Say hello").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");
    }
}
