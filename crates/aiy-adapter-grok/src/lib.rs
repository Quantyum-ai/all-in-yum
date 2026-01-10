//! # aiy-adapter-grok
//!
//! Grok (xAI) adapter for the All-in-Yum multi-agent consensus pipeline.
//!
//! This crate provides a secure integration with xAI's Grok API, implementing
//! the `AgentAdapter` trait and enforcing prompt injection defenses.

pub mod client;
pub mod error;
pub mod models;
pub mod types;

pub use client::GrokClient;
pub use error::GrokError;
pub use models::GrokModel;
pub use types::{ChatMessage, ChatRequest, ChatResponse};

use aiy_adapters::{AdapterError, AgentAdapter, AgentReview};
use async_trait::async_trait;

/// Grok adapter implementing the AgentAdapter trait
pub struct GrokAdapter {
    client: GrokClient,
}

impl GrokAdapter {
    /// Create a new GrokAdapter with a pre-configured client
    pub fn new(client: GrokClient) -> Self {
        Self { client }
    }

    /// Generate text from a prompt (inherent async method)
    pub async fn generate_text(&self, prompt: &str) -> Result<String, GrokError> {
        self.client.generate_text(prompt).await
    }

    /// Review an artifact with prompt injection defenses
    ///
    /// This method:
    /// 1. Sanitizes the artifact content
    /// 2. Builds a secure review prompt
    /// 3. Validates the response
    /// 4. Enforces schema compliance
    pub async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, GrokError> {
        self.client.review_artifact(artifact).await
    }
}

#[async_trait]
impl AgentAdapter for GrokAdapter {
    fn id(&self) -> &str {
        "grok"
    }

    fn display_name(&self) -> &str {
        "Grok (xAI)"
    }

    async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, AdapterError> {
        self.client
            .review_artifact(artifact)
            .await
            .map_err(|e| AdapterError::new(e.to_sanitized_string()))
    }
}
