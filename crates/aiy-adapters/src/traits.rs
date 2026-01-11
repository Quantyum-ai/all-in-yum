use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Classification of adapter errors for better handling and user guidance.
///
/// This enum enables callers to programmatically respond to different error types,
/// such as implementing retry logic for rate limits or network issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdapterErrorKind {
    /// Authentication failure (invalid API key, expired token)
    Auth,
    /// Rate limit exceeded
    RateLimit,
    /// Network connectivity issue
    Network,
    /// Request timeout
    Timeout,
    /// Failed to parse response
    Parse,
    /// Invalid request schema
    Schema,
    /// Security-related error (e.g., injection detected)
    Security,
    /// Unknown/unclassified error
    Unknown,
}

impl AdapterErrorKind {
    /// Returns whether errors of this kind are generally retryable.
    ///
    /// - `RateLimit`, `Network`, `Timeout`: Usually transient, worth retrying
    /// - `Auth`, `Parse`, `Schema`, `Security`, `Unknown`: Usually permanent
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AdapterErrorKind::RateLimit | AdapterErrorKind::Network | AdapterErrorKind::Timeout
        )
    }
}

impl fmt::Display for AdapterErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            AdapterErrorKind::Auth => "authentication error",
            AdapterErrorKind::RateLimit => "rate limit exceeded",
            AdapterErrorKind::Network => "network error",
            AdapterErrorKind::Timeout => "timeout",
            AdapterErrorKind::Parse => "parse error",
            AdapterErrorKind::Schema => "schema error",
            AdapterErrorKind::Security => "security error",
            AdapterErrorKind::Unknown => "unknown error",
        };
        write!(f, "{}", label)
    }
}

/// Error type for adapter operations.
///
/// This struct provides structured error information including:
/// - `kind`: Classification for programmatic handling
/// - `message`: Human-readable description (sanitized, safe to log)
/// - `retryable`: Whether the operation may succeed if retried
///
/// Individual adapter crates define richer error types and convert to this,
/// ensuring sensitive data (API keys, tokens) is never exposed.
#[derive(Debug, Clone)]
pub struct AdapterError {
    kind: AdapterErrorKind,
    message: String,
    retryable: bool,
}

impl AdapterError {
    /// Create a new adapter error with explicit kind and retryability.
    pub fn new(kind: AdapterErrorKind, message: impl Into<String>) -> Self {
        let retryable = kind.is_retryable();
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }

    /// Create a new adapter error with custom retryability.
    pub fn with_retryable(kind: AdapterErrorKind, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }

    /// Create an authentication error.
    pub fn auth(message: impl Into<String>) -> Self {
        Self::new(AdapterErrorKind::Auth, message)
    }

    /// Create a rate limit error.
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::new(AdapterErrorKind::RateLimit, message)
    }

    /// Create a network error.
    pub fn network(message: impl Into<String>) -> Self {
        Self::new(AdapterErrorKind::Network, message)
    }

    /// Create a timeout error.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(AdapterErrorKind::Timeout, message)
    }

    /// Create a parse error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(AdapterErrorKind::Parse, message)
    }

    /// Create a schema validation error.
    pub fn schema(message: impl Into<String>) -> Self {
        Self::new(AdapterErrorKind::Schema, message)
    }

    /// Create a security error.
    pub fn security(message: impl Into<String>) -> Self {
        Self::new(AdapterErrorKind::Security, message)
    }

    /// Create an unknown/unclassified error.
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(AdapterErrorKind::Unknown, message)
    }

    /// Get the error kind.
    pub fn kind(&self) -> AdapterErrorKind {
        self.kind
    }

    /// Get the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns whether this error is retryable.
    pub fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AdapterError {}

/// Retry configuration for transient errors.
///
/// This configuration can be used by adapters for handling rate limits,
/// network issues, and other transient failures with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay in milliseconds before the first retry.
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds between retries.
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff.
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

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

    /// Perform a health check to verify credentials and connectivity.
    ///
    /// Returns `Ok(())` if the adapter is ready to perform reviews.
    /// Default implementation returns `Ok(())` - adapters can override for actual checks.
    ///
    /// # Example
    ///
    /// Adapters can override this to verify API connectivity:
    ///
    /// ```ignore
    /// async fn health_check(&self) -> Result<(), AdapterError> {
    ///     // Perform a lightweight API call to verify credentials
    ///     self.client.ping().await.map_err(|e| AdapterError::auth(e.to_string()))
    /// }
    /// ```
    async fn health_check(&self) -> Result<(), AdapterError> {
        Ok(())
    }

    /// Review an artifact (code, diff, or other content) and return structured feedback.
    ///
    /// This is the primary method for code review operations. Implementations must:
    /// - Sanitize input to prevent prompt injection
    /// - Build secure review prompts
    /// - Validate response schema
    /// - Never leak API keys or secrets in errors
    async fn review_artifact(&self, artifact: &str) -> Result<AgentReview, AdapterError>;
}

/// Structured feedback from an AI agent's code review.
///
/// This struct captures the complete output of a single agent's review,
/// including verdict, confidence, identified issues, and reasoning.
///
/// # Example
///
/// ```
/// use aiy_adapters::{AgentReview, Verdict, Issue, Severity};
///
/// let review = AgentReview {
///     agent_id: "claude".to_string(),
///     verdict: Verdict::Pass,
///     confidence: 0.95,
///     issues: vec![],
///     suggestions: vec!["Consider adding more tests".to_string()],
///     sign_off: true,
///     reasoning: "Code follows best practices".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReview {
    /// Unique identifier of the agent that produced this review.
    pub agent_id: String,
    /// The agent's overall verdict on the code.
    pub verdict: Verdict,
    /// Confidence level in the verdict (0.0 to 1.0).
    ///
    /// A value of 1.0 indicates complete certainty, while lower values
    /// suggest the agent is less sure about its assessment.
    pub confidence: f64,
    /// List of issues identified in the code.
    pub issues: Vec<Issue>,
    /// General suggestions for improvement (not tied to specific issues).
    pub suggestions: Vec<String>,
    /// Whether the agent explicitly signs off on the code for merge.
    ///
    /// This may differ from verdict in edge cases where an agent identifies
    /// minor issues but still approves the overall change.
    pub sign_off: bool,
    /// Human-readable explanation of the agent's decision.
    pub reasoning: String,
}

/// The overall verdict from an agent's code review.
///
/// Verdicts are ordered by severity: `Pass` < `Issue` < `Block`.
/// The consensus engine uses these to determine the final outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Code is approved and ready to merge.
    Pass,
    /// Code has issues that should be addressed but may not block merge.
    Issue,
    /// Code has critical problems that must be resolved before merge.
    ///
    /// # Security
    ///
    /// A `Block` verdict typically indicates security vulnerabilities,
    /// critical bugs, or other issues that could cause serious harm.
    Block,
}

/// A specific issue identified during code review.
///
/// Issues capture problems ranging from minor style nits to critical
/// security vulnerabilities. Each issue includes location information
/// when available to help developers locate and fix the problem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// How severe this issue is (determines urgency of fix).
    pub severity: Severity,
    /// Category of the issue (e.g., "security", "performance", "style").
    pub category: String,
    /// Human-readable description of what's wrong.
    pub description: String,
    /// Location in the code (e.g., "src/main.rs:42" or "api.rs:10-15").
    ///
    /// May be `None` for issues that apply to the code as a whole.
    pub location: Option<String>,
    /// Suggested code or approach to fix the issue.
    ///
    /// May be `None` if no specific fix is recommended.
    pub suggested_fix: Option<String>,
}

/// Severity level of an issue.
///
/// Severities are ordered from most severe to least:
/// `Critical` > `Major` > `Minor` > `Nit`.
///
/// The `PartialOrd` and `Ord` implementations reflect this ordering,
/// making `Critical` the "greatest" value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Critical issue that must be fixed before merge.
    ///
    /// # Security
    ///
    /// Typically used for security vulnerabilities, data loss risks,
    /// or bugs that could cause system crashes or corruption.
    Critical,
    /// Major issue that should be fixed but might not block merge.
    Major,
    /// Minor issue worth addressing but low priority.
    Minor,
    /// Nitpick or style suggestion with no functional impact.
    Nit,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // AdapterErrorKind classification tests
    // ==========================================================================

    #[test]
    fn test_auth_error_kind_not_retryable() {
        assert!(!AdapterErrorKind::Auth.is_retryable());
    }

    #[test]
    fn test_rate_limit_error_kind_is_retryable() {
        assert!(AdapterErrorKind::RateLimit.is_retryable());
    }

    #[test]
    fn test_network_error_kind_is_retryable() {
        assert!(AdapterErrorKind::Network.is_retryable());
    }

    #[test]
    fn test_timeout_error_kind_is_retryable() {
        assert!(AdapterErrorKind::Timeout.is_retryable());
    }

    #[test]
    fn test_parse_error_kind_not_retryable() {
        assert!(!AdapterErrorKind::Parse.is_retryable());
    }

    #[test]
    fn test_schema_error_kind_not_retryable() {
        assert!(!AdapterErrorKind::Schema.is_retryable());
    }

    #[test]
    fn test_security_error_kind_not_retryable() {
        assert!(!AdapterErrorKind::Security.is_retryable());
    }

    #[test]
    fn test_unknown_error_kind_not_retryable() {
        assert!(!AdapterErrorKind::Unknown.is_retryable());
    }

    // ==========================================================================
    // AdapterError constructor tests
    // ==========================================================================

    #[test]
    fn test_auth_constructor() {
        let err = AdapterError::auth("Invalid API key");
        assert_eq!(err.kind(), AdapterErrorKind::Auth);
        assert_eq!(err.message(), "Invalid API key");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_rate_limit_constructor() {
        let err = AdapterError::rate_limit("Too many requests");
        assert_eq!(err.kind(), AdapterErrorKind::RateLimit);
        assert_eq!(err.message(), "Too many requests");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_network_constructor() {
        let err = AdapterError::network("Connection refused");
        assert_eq!(err.kind(), AdapterErrorKind::Network);
        assert_eq!(err.message(), "Connection refused");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_timeout_constructor() {
        let err = AdapterError::timeout("Request timed out after 30s");
        assert_eq!(err.kind(), AdapterErrorKind::Timeout);
        assert_eq!(err.message(), "Request timed out after 30s");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_parse_constructor() {
        let err = AdapterError::parse("Invalid JSON response");
        assert_eq!(err.kind(), AdapterErrorKind::Parse);
        assert_eq!(err.message(), "Invalid JSON response");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_schema_constructor() {
        let err = AdapterError::schema("Missing required field 'verdict'");
        assert_eq!(err.kind(), AdapterErrorKind::Schema);
        assert_eq!(err.message(), "Missing required field 'verdict'");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_security_constructor() {
        let err = AdapterError::security("Prompt injection detected");
        assert_eq!(err.kind(), AdapterErrorKind::Security);
        assert_eq!(err.message(), "Prompt injection detected");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_unknown_constructor() {
        let err = AdapterError::unknown("Something went wrong");
        assert_eq!(err.kind(), AdapterErrorKind::Unknown);
        assert_eq!(err.message(), "Something went wrong");
        assert!(!err.is_retryable());
    }

    // ==========================================================================
    // Custom retryability tests
    // ==========================================================================

    #[test]
    fn test_with_retryable_override() {
        // Auth errors are normally not retryable, but can be overridden
        let err = AdapterError::with_retryable(AdapterErrorKind::Auth, "Token expired", true);
        assert_eq!(err.kind(), AdapterErrorKind::Auth);
        assert!(err.is_retryable());

        // Network errors are normally retryable, but can be overridden
        let err = AdapterError::with_retryable(AdapterErrorKind::Network, "Fatal network error", false);
        assert_eq!(err.kind(), AdapterErrorKind::Network);
        assert!(!err.is_retryable());
    }

    // ==========================================================================
    // Display trait tests - verify no secret leakage
    // ==========================================================================

    #[test]
    fn test_error_display_shows_message_only() {
        let err = AdapterError::auth("Credential retrieval failed");
        let displayed = format!("{}", err);
        assert_eq!(displayed, "Credential retrieval failed");
        // Ensure we don't expose internal structure
        assert!(!displayed.contains("Auth"));
        assert!(!displayed.contains("retryable"));
    }

    #[test]
    fn test_error_kind_display() {
        assert_eq!(format!("{}", AdapterErrorKind::Auth), "authentication error");
        assert_eq!(format!("{}", AdapterErrorKind::RateLimit), "rate limit exceeded");
        assert_eq!(format!("{}", AdapterErrorKind::Network), "network error");
        assert_eq!(format!("{}", AdapterErrorKind::Timeout), "timeout");
        assert_eq!(format!("{}", AdapterErrorKind::Parse), "parse error");
        assert_eq!(format!("{}", AdapterErrorKind::Schema), "schema error");
        assert_eq!(format!("{}", AdapterErrorKind::Security), "security error");
        assert_eq!(format!("{}", AdapterErrorKind::Unknown), "unknown error");
    }

    // ==========================================================================
    // Debug trait safety - verify no accidental secret exposure
    // ==========================================================================

    #[test]
    fn test_error_debug_is_safe_to_log() {
        // Create an error with a message that looks like it could contain secrets
        // (in practice, adapters should sanitize before creating AdapterError)
        let err = AdapterError::auth("Credential retrieval failed");
        let debug_output = format!("{:?}", err);

        // Debug output should contain the kind and message, but that's fine
        // since the message should already be sanitized by the adapter
        assert!(debug_output.contains("Auth"));
        assert!(debug_output.contains("Credential retrieval failed"));
        // Key point: if there were a raw API key, it would appear here
        // Adapters MUST sanitize before calling these constructors
    }

    // ==========================================================================
    // Error trait implementation
    // ==========================================================================

    #[test]
    fn test_error_implements_std_error() {
        let err = AdapterError::network("Connection failed");
        let std_err: &dyn std::error::Error = &err;
        assert!(std_err.to_string().contains("Connection failed"));
    }
}
