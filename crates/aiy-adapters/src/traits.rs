use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Error type for adapter operations.
///
/// This is intentionally minimal - a message wrapper without extra dependencies.
/// Individual adapter crates can define richer error types and convert to this.
#[derive(Debug, Clone)]
pub struct AdapterError {
    message: String,
}

impl AdapterError {
    /// Create a new adapter error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Get the error message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AdapterError {}

/// Trait for all AI agent adapters in the consensus pipeline.
///
/// Each adapter (Grok, Claude, Gemini, Codex) implements this trait to provide
/// a unified interface for code review operations.
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Unique identifier for this agent (e.g., "grok", "claude", "gemini").
    fn id(&self) -> &str;

    /// Human-readable display name (e.g., "Grok (xAI)", "Claude (Anthropic)").
    fn display_name(&self) -> &str;

    /// Review an artifact (code, diff, or other content) and return structured feedback.
    ///
    /// This is the primary method for code review operations. Implementations must:
    /// - Sanitize input to prevent prompt injection
    /// - Build secure review prompts
    /// - Validate response schema
    /// - Never leak API keys or secrets in errors
    async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, AdapterError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReview {
    pub agent_id: String,
    pub verdict: Verdict,
    pub confidence: f64,
    pub issues: Vec<Issue>,
    pub suggestions: Vec<String>,
    pub sign_off: bool,
    pub reasoning: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Issue,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub category: String,
    pub description: String,
    pub location: Option<String>,
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Major,
    Minor,
    Nit,
}
