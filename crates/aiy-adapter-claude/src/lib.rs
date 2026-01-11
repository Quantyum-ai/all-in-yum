//! # aiy-adapter-claude
//!
//! Claude (Anthropic) adapter for the All-in-Yum multi-agent consensus pipeline.
//!
//! This crate provides a secure integration with Anthropic's Claude API, implementing
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
//! use aiy_adapter_claude::{ClaudeClient, client::MockTransport};
//!
//! let transport = Arc::new(MockTransport::with_canned_response(response));
//! let client = ClaudeClient::new_with_mock(credential_manager, transport);
//! ```
//!
//! ### With Real HTTP Transport (requires `http` feature)
//! ```rust,ignore
//! use aiy_adapter_claude::ClaudeClient;
//!
//! let client = ClaudeClient::new_with_http(credential_manager)?;
//! ```

pub mod client;
pub mod error;
pub mod models;
pub mod transport;
pub mod types;

pub use client::ClaudeClient;
pub use error::ClaudeError;
pub use models::ClaudeModel;
pub use transport::{HttpTransport, MockTransport};
pub use types::{Message, MessageContent, MessageRole, MessagesRequest, MessagesResponse};

#[cfg(feature = "http")]
pub use transport::ReqwestTransport;

use aiy_adapters::{AdapterError, AgentAdapter, AgentReview};
use async_trait::async_trait;

/// Claude adapter implementing the AgentAdapter trait
pub struct ClaudeAdapter {
    client: ClaudeClient,
}

impl ClaudeAdapter {
    /// Create a new ClaudeAdapter with a pre-configured client
    pub fn new(client: ClaudeClient) -> Self {
        Self { client }
    }

    /// Generate text from a prompt (inherent async method)
    pub async fn generate_text(&self, prompt: &str) -> Result<String, ClaudeError> {
        self.client.generate_text(prompt).await
    }

    /// Review an artifact with prompt injection defenses
    ///
    /// This method:
    /// 1. Sanitizes the artifact content
    /// 2. Builds a secure review prompt
    /// 3. Validates the response
    /// 4. Enforces schema compliance
    pub async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, ClaudeError> {
        self.client.review_artifact(artifact).await
    }
}

#[async_trait]
impl AgentAdapter for ClaudeAdapter {
    fn id(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude (Anthropic)"
    }

    async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, AdapterError> {
        self.client
            .review_artifact(artifact)
            .await
            .map_err(|e| e.to_adapter_error())
    }
}
