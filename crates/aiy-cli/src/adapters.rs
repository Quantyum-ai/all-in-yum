//! Adapter Factory Module
//!
//! This module provides factory functions to create AI agent adapters for both
//! review and ask operations. It centralizes adapter creation logic and provides
//! a unified interface for working with multiple AI providers.
//!
//! ## Supported Agents
//!
//! - **Grok** (xAI): Fast, efficient reviews with grok-4-1-fast
//! - **Claude** (Anthropic): High-quality analysis with claude-sonnet-4
//! - **Gemini** (Google): Multi-modal capabilities with gemini-1.5-pro
//! - **Codex** (OpenAI): Code-focused analysis with gpt-4o
//!
//! ## Usage
//!
//! ### For Review Operations
//! ```rust,ignore
//! use aiy_cli::adapters::create_review_adapter;
//!
//! let adapter = create_review_adapter("grok", credential_manager).await?;
//! let review = adapter.review_artifact(&code).await?;
//! ```
//!
//! ### For Ask Operations
//! ```rust,ignore
//! use aiy_cli::adapters::{create_ask_adapter, AskAdapter};
//!
//! let adapter = create_ask_adapter("claude", credential_manager).await?;
//! let response = adapter.generate_text("Explain this code").await?;
//! ```
//!
//! ## Feature Flags
//!
//! - `http`: Enables real HTTP transport for production API calls
//! - Without `http`: Uses mock transport (for testing only)

use aiy_adapters::AgentAdapter;
use aiy_core::security::CredentialManager;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;

// Import all adapter crates
use aiy_adapter_claude::{ClaudeAdapter, ClaudeClient, ClaudeError};
use aiy_adapter_codex::{CodexAdapter, CodexClient, CodexError};
use aiy_adapter_gemini::{GeminiAdapter, GeminiClient, GeminiError};
use aiy_adapter_grok::{GrokAdapter, GrokClient, GrokError};

// Import mock transports
// Note: Gemini doesn't have HTTP feature yet, so we always use mock for it
// We need HttpTransport trait in scope for Arc<dyn HttpTransport> coercion
use aiy_adapter_gemini::{HttpTransport as _, MockTransport as GeminiMockTransport};

#[cfg(not(feature = "http"))]
use aiy_adapter_claude::MockTransport as ClaudeMockTransport;
#[cfg(not(feature = "http"))]
use aiy_adapter_codex::MockTransport as CodexMockTransport;
#[cfg(not(feature = "http"))]
use aiy_adapter_grok::MockTransport as GrokMockTransport;

use crate::registry::{get_agent, AGENTS};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during adapter creation
#[derive(Debug)]
pub enum AdapterCreationError {
    /// The specified agent ID is not recognized
    UnknownAgent(String),

    /// Credentials for the agent are not configured
    MissingCredentials(String),

    /// Failed to create the adapter (transport, client, etc.)
    CreationFailed(String),

    /// HTTP feature not enabled (only mock transport available)
    HttpNotEnabled(String),

    /// Grok-specific error during creation
    Grok(GrokError),

    /// Claude-specific error during creation
    Claude(ClaudeError),

    /// Gemini-specific error during creation
    Gemini(GeminiError),

    /// Codex-specific error during creation
    Codex(CodexError),
}

impl fmt::Display for AdapterCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAgent(id) => write!(f, "Unknown agent: '{}'. Valid agents: {}", id, valid_agents_list()),
            Self::MissingCredentials(id) => write!(
                f,
                "No credentials found for agent '{}'. Set up credentials with: aiy credentials set {}",
                id,
                get_credential_provider(id)
            ),
            Self::CreationFailed(msg) => write!(f, "Failed to create adapter: {}", msg),
            Self::HttpNotEnabled(id) => write!(
                f,
                "HTTP transport not enabled for '{}'. Build with `--features http` for real API calls.",
                id
            ),
            Self::Grok(e) => write!(f, "Grok adapter error: {}", e.to_sanitized_string()),
            Self::Claude(e) => write!(f, "Claude adapter error: {}", e.to_sanitized_string()),
            Self::Gemini(e) => write!(f, "Gemini adapter error: {}", e.to_sanitized_string()),
            Self::Codex(e) => write!(f, "Codex adapter error: {}", e.to_sanitized_string()),
        }
    }
}

impl std::error::Error for AdapterCreationError {}

impl From<GrokError> for AdapterCreationError {
    fn from(err: GrokError) -> Self {
        AdapterCreationError::Grok(err)
    }
}

impl From<ClaudeError> for AdapterCreationError {
    fn from(err: ClaudeError) -> Self {
        AdapterCreationError::Claude(err)
    }
}

impl From<GeminiError> for AdapterCreationError {
    fn from(err: GeminiError) -> Self {
        AdapterCreationError::Gemini(err)
    }
}

impl From<CodexError> for AdapterCreationError {
    fn from(err: CodexError) -> Self {
        AdapterCreationError::Codex(err)
    }
}

// ============================================================================
// AskAdapter Enum
// ============================================================================

/// Unified adapter enum for ask operations
///
/// This enum wraps all supported adapters and provides a common interface
/// for text generation. Unlike `Box<dyn AgentAdapter>`, this allows access
/// to adapter-specific `generate_text()` methods.
pub enum AskAdapter {
    /// Grok (xAI) adapter
    Grok(GrokAdapter),
    /// Claude (Anthropic) adapter
    Claude(ClaudeAdapter),
    /// Gemini (Google) adapter
    Gemini(GeminiAdapter),
    /// Codex (OpenAI) adapter
    Codex(CodexAdapter),
}

impl AskAdapter {
    /// Generate text from a prompt using the underlying adapter
    ///
    /// This method dispatches to the correct adapter's `generate_text()` method
    /// and converts any adapter-specific errors to `AdapterCreationError`.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The text prompt to send to the AI agent
    ///
    /// # Returns
    ///
    /// The generated text response, or an error if the request fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let adapter = create_ask_adapter("grok", cm).await?;
    /// let response = adapter.generate_text("What is Rust?").await?;
    /// println!("{}", response);
    /// ```
    pub async fn generate_text(&self, prompt: &str) -> Result<String, AdapterCreationError> {
        match self {
            AskAdapter::Grok(adapter) => adapter
                .generate_text(prompt)
                .await
                .map_err(AdapterCreationError::from),
            AskAdapter::Claude(adapter) => adapter
                .generate_text(prompt)
                .await
                .map_err(AdapterCreationError::from),
            AskAdapter::Gemini(adapter) => adapter
                .generate_text(prompt)
                .await
                .map_err(AdapterCreationError::from),
            AskAdapter::Codex(adapter) => adapter
                .generate_text(prompt)
                .await
                .map_err(AdapterCreationError::from),
        }
    }

    /// Get the agent ID for this adapter
    pub fn agent_id(&self) -> &str {
        match self {
            AskAdapter::Grok(_) => "grok",
            AskAdapter::Claude(_) => "claude",
            AskAdapter::Gemini(_) => "gemini",
            AskAdapter::Codex(_) => "codex",
        }
    }

    /// Get the display name for this adapter
    pub fn display_name(&self) -> &str {
        match self {
            AskAdapter::Grok(_) => "Grok (xAI)",
            AskAdapter::Claude(_) => "Claude (Anthropic)",
            AskAdapter::Gemini(_) => "Gemini (Google)",
            AskAdapter::Codex(_) => "Codex (OpenAI)",
        }
    }

    /// Get the model name used by this adapter
    pub fn model_name(&self) -> &str {
        match self {
            AskAdapter::Grok(_) => "grok-4-1-fast",
            AskAdapter::Claude(_) => "claude-sonnet-4-20250514",
            AskAdapter::Gemini(_) => "gemini-1.5-pro-latest",
            AskAdapter::Codex(_) => "gpt-4o",
        }
    }
}

// ============================================================================
// Factory Functions (HTTP-enabled)
// ============================================================================

/// Create a review adapter for the specified agent
///
/// This factory function creates an adapter that implements the `AgentAdapter`
/// trait, suitable for code review operations.
///
/// # Arguments
///
/// * `agent_id` - The agent identifier (e.g., "grok", "claude", "gemini", "codex")
/// * `credential_manager` - Shared credential manager for API key retrieval
///
/// # Returns
///
/// A boxed `AgentAdapter` trait object, or an error if creation fails.
///
/// # Errors
///
/// - `UnknownAgent` if the agent_id is not recognized
/// - `MissingCredentials` if no API key is configured for the agent
/// - Adapter-specific errors during client creation
///
/// # Example
///
/// ```rust,ignore
/// let cm = Arc::new(Mutex::new(CredentialManager::new(...)?));
/// let adapter = create_review_adapter("claude", cm).await?;
/// let review = adapter.review_artifact(&code).await?;
/// ```
#[cfg(feature = "http")]
pub async fn create_review_adapter(
    agent_id: &str,
    credential_manager: Arc<Mutex<CredentialManager>>,
) -> Result<Box<dyn AgentAdapter>, AdapterCreationError> {
    // Validate agent ID
    if get_agent(agent_id).is_none() {
        return Err(AdapterCreationError::UnknownAgent(agent_id.to_string()));
    }

    // Check credentials exist
    if !has_credentials(agent_id, &credential_manager).await {
        return Err(AdapterCreationError::MissingCredentials(
            agent_id.to_string(),
        ));
    }

    // Create adapter based on agent ID
    match agent_id {
        "grok" => {
            let client = GrokClient::new_with_http(credential_manager)?;
            Ok(Box::new(GrokAdapter::new(client)))
        }
        "claude" => {
            let client = ClaudeClient::new_with_http(credential_manager)?;
            Ok(Box::new(ClaudeAdapter::new(client)))
        }
        "gemini" => {
            // Gemini doesn't have new_with_http yet, use mock for now
            // TODO: Implement HTTP transport for Gemini
            let mock_transport = Arc::new(GeminiMockTransport::new());
            let client = GeminiClient::new_with_mock(credential_manager, mock_transport);
            Ok(Box::new(GeminiAdapter::new(client)))
        }
        "codex" => {
            let client = CodexClient::new_with_http(credential_manager)?;
            Ok(Box::new(CodexAdapter::new(client)))
        }
        _ => Err(AdapterCreationError::UnknownAgent(agent_id.to_string())),
    }
}

/// Create an ask adapter for the specified agent
///
/// This factory function creates an `AskAdapter` enum variant suitable for
/// text generation operations (the `ask` command).
///
/// # Arguments
///
/// * `agent_id` - The agent identifier (e.g., "grok", "claude", "gemini", "codex")
/// * `credential_manager` - Shared credential manager for API key retrieval
///
/// # Returns
///
/// An `AskAdapter` enum variant, or an error if creation fails.
///
/// # Errors
///
/// - `UnknownAgent` if the agent_id is not recognized
/// - `MissingCredentials` if no API key is configured for the agent
/// - Adapter-specific errors during client creation
///
/// # Example
///
/// ```rust,ignore
/// let cm = Arc::new(Mutex::new(CredentialManager::new(...)?));
/// let adapter = create_ask_adapter("grok", cm).await?;
/// let response = adapter.generate_text("Hello!").await?;
/// ```
#[cfg(feature = "http")]
pub async fn create_ask_adapter(
    agent_id: &str,
    credential_manager: Arc<Mutex<CredentialManager>>,
) -> Result<AskAdapter, AdapterCreationError> {
    // Validate agent ID
    if get_agent(agent_id).is_none() {
        return Err(AdapterCreationError::UnknownAgent(agent_id.to_string()));
    }

    // Check credentials exist
    if !has_credentials(agent_id, &credential_manager).await {
        return Err(AdapterCreationError::MissingCredentials(
            agent_id.to_string(),
        ));
    }

    // Create adapter based on agent ID
    match agent_id {
        "grok" => {
            let client = GrokClient::new_with_http(credential_manager)?;
            Ok(AskAdapter::Grok(GrokAdapter::new(client)))
        }
        "claude" => {
            let client = ClaudeClient::new_with_http(credential_manager)?;
            Ok(AskAdapter::Claude(ClaudeAdapter::new(client)))
        }
        "gemini" => {
            // Gemini doesn't have new_with_http yet, use mock for now
            // TODO: Implement HTTP transport for Gemini
            let mock_transport = Arc::new(GeminiMockTransport::new());
            let client = GeminiClient::new_with_mock(credential_manager, mock_transport);
            Ok(AskAdapter::Gemini(GeminiAdapter::new(client)))
        }
        "codex" => {
            let client = CodexClient::new_with_http(credential_manager)?;
            Ok(AskAdapter::Codex(CodexAdapter::new(client)))
        }
        _ => Err(AdapterCreationError::UnknownAgent(agent_id.to_string())),
    }
}

// ============================================================================
// Factory Functions (Mock-only, no HTTP feature)
// ============================================================================

/// Create a review adapter for the specified agent (mock transport only)
///
/// This version is used when the `http` feature is not enabled.
/// It creates adapters with mock transports for testing purposes.
#[cfg(not(feature = "http"))]
pub async fn create_review_adapter(
    agent_id: &str,
    credential_manager: Arc<Mutex<CredentialManager>>,
) -> Result<Box<dyn AgentAdapter>, AdapterCreationError> {
    // Validate agent ID
    if get_agent(agent_id).is_none() {
        return Err(AdapterCreationError::UnknownAgent(agent_id.to_string()));
    }

    // Check credentials exist
    if !has_credentials(agent_id, &credential_manager).await {
        return Err(AdapterCreationError::MissingCredentials(
            agent_id.to_string(),
        ));
    }

    // Create adapter with mock transport
    match agent_id {
        "grok" => {
            let mock_transport = Arc::new(GrokMockTransport::with_canned_response(
                mock_review_response("grok"),
            ));
            let client = GrokClient::new_with_mock(credential_manager, mock_transport);
            Ok(Box::new(GrokAdapter::new(client)))
        }
        "claude" => {
            let mock_transport = Arc::new(ClaudeMockTransport::with_canned_response(
                mock_claude_review_response(),
            ));
            let client = ClaudeClient::new_with_mock(credential_manager, mock_transport);
            Ok(Box::new(ClaudeAdapter::new(client)))
        }
        "gemini" => {
            let mock_transport = Arc::new(GeminiMockTransport::with_canned_response(
                mock_gemini_review_response(),
            ));
            let client = GeminiClient::new_with_mock(credential_manager, mock_transport);
            Ok(Box::new(GeminiAdapter::new(client)))
        }
        "codex" => {
            let mock_transport = Arc::new(CodexMockTransport::with_canned_response(
                mock_review_response("codex"),
            ));
            let client = CodexClient::new_with_mock(credential_manager, mock_transport);
            Ok(Box::new(CodexAdapter::new(client)))
        }
        _ => Err(AdapterCreationError::UnknownAgent(agent_id.to_string())),
    }
}

/// Create an ask adapter for the specified agent (mock transport only)
///
/// This version is used when the `http` feature is not enabled.
/// It creates adapters with mock transports for testing purposes.
#[cfg(not(feature = "http"))]
pub async fn create_ask_adapter(
    agent_id: &str,
    credential_manager: Arc<Mutex<CredentialManager>>,
) -> Result<AskAdapter, AdapterCreationError> {
    // Validate agent ID
    if get_agent(agent_id).is_none() {
        return Err(AdapterCreationError::UnknownAgent(agent_id.to_string()));
    }

    // Check credentials exist
    if !has_credentials(agent_id, &credential_manager).await {
        return Err(AdapterCreationError::MissingCredentials(
            agent_id.to_string(),
        ));
    }

    // Create adapter with mock transport
    match agent_id {
        "grok" => {
            let mock_transport =
                Arc::new(GrokMockTransport::with_canned_response(mock_text_response()));
            let client = GrokClient::new_with_mock(credential_manager, mock_transport);
            Ok(AskAdapter::Grok(GrokAdapter::new(client)))
        }
        "claude" => {
            let mock_transport = Arc::new(ClaudeMockTransport::with_canned_response(
                mock_claude_text_response(),
            ));
            let client = ClaudeClient::new_with_mock(credential_manager, mock_transport);
            Ok(AskAdapter::Claude(ClaudeAdapter::new(client)))
        }
        "gemini" => {
            let mock_transport = Arc::new(GeminiMockTransport::with_canned_response(
                mock_gemini_text_response(),
            ));
            let client = GeminiClient::new_with_mock(credential_manager, mock_transport);
            Ok(AskAdapter::Gemini(GeminiAdapter::new(client)))
        }
        "codex" => {
            let mock_transport = Arc::new(CodexMockTransport::with_canned_response(
                mock_text_response(),
            ));
            let client = CodexClient::new_with_mock(credential_manager, mock_transport);
            Ok(AskAdapter::Codex(CodexAdapter::new(client)))
        }
        _ => Err(AdapterCreationError::UnknownAgent(agent_id.to_string())),
    }
}

// ============================================================================
// Credential Checking
// ============================================================================

/// Check if credentials are available for the specified agent
///
/// This function checks both the canonical credential provider name and any
/// fallback names (e.g., "xai" and "grok" for Grok adapter).
///
/// # Arguments
///
/// * `agent_id` - The agent identifier
/// * `cm` - Shared credential manager to check
///
/// # Returns
///
/// `true` if credentials are available, `false` otherwise.
///
/// # Example
///
/// ```rust,ignore
/// if has_credentials("grok", &cm).await {
///     println!("Grok is ready to use!");
/// }
/// ```
pub async fn has_credentials(agent_id: &str, cm: &Arc<Mutex<CredentialManager>>) -> bool {
    let manager = cm.lock().await;

    // Get the primary and fallback credential provider names
    let (primary, fallback) = get_credential_providers(agent_id);

    // Check primary provider
    if manager.get_key(primary).is_ok() {
        return true;
    }

    // Check fallback provider if different
    if let Some(fallback_name) = fallback {
        if manager.get_key(fallback_name).is_ok() {
            return true;
        }
    }

    false
}

/// Get all agents with valid credentials
///
/// Returns a list of agent IDs that have credentials configured.
///
/// # Arguments
///
/// * `cm` - Shared credential manager to check
///
/// # Returns
///
/// Vector of agent IDs with valid credentials.
pub async fn get_available_agents(cm: &Arc<Mutex<CredentialManager>>) -> Vec<&'static str> {
    let mut available = Vec::new();

    for agent in AGENTS {
        if has_credentials(agent.id, cm).await {
            available.push(agent.id);
        }
    }

    available
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the credential provider names for an agent
///
/// Returns (primary, optional_fallback) tuple of provider names.
fn get_credential_providers(agent_id: &str) -> (&'static str, Option<&'static str>) {
    match agent_id {
        "grok" => ("xai", Some("grok")),
        "claude" => ("anthropic", Some("claude")),
        "gemini" => ("google", Some("gemini")),
        "codex" => ("openai", Some("codex")),
        _ => ("unknown", None),
    }
}

/// Get the primary credential provider for an agent
fn get_credential_provider(agent_id: &str) -> &'static str {
    get_credential_providers(agent_id).0
}

/// Get a comma-separated list of valid agent IDs
fn valid_agents_list() -> String {
    AGENTS.iter().map(|a| a.id).collect::<Vec<_>>().join(", ")
}

// ============================================================================
// Mock Response Generators (for non-HTTP mode)
// ============================================================================

/// Generate a mock review response for Grok/Codex (OpenAI-compatible format)
#[cfg(not(feature = "http"))]
fn mock_review_response(agent_id: &str) -> String {
    format!(
        r#"{{
            "id": "mock-{}-id",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "mock-model",
            "choices": [{{
                "index": 0,
                "message": {{
                    "role": "assistant",
                    "content": "{{\"agent_id\":\"{}\",\"verdict\":\"pass\",\"confidence\":0.95,\"issues\":[],\"suggestions\":[\"Consider adding more documentation\"],\"sign_off\":true,\"reasoning\":\"Code looks clean and follows best practices.\"}}"
                }},
                "finish_reason": "stop"
            }}]
        }}"#,
        agent_id, agent_id
    )
}

/// Generate a mock review response for Claude (Anthropic format)
#[cfg(not(feature = "http"))]
fn mock_claude_review_response() -> String {
    r#"{
        "id": "msg_mock",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "{\"agent_id\":\"claude\",\"verdict\":\"pass\",\"confidence\":0.92,\"issues\":[],\"suggestions\":[\"Code is well-structured\"],\"sign_off\":true,\"reasoning\":\"The code follows Anthropic best practices.\"}"
            }
        ],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 100,
            "output_tokens": 50
        }
    }"#
    .to_string()
}

/// Generate a mock review response for Gemini (Google format)
#[cfg(not(feature = "http"))]
fn mock_gemini_review_response() -> String {
    r#"{
        "candidates": [{
            "content": {
                "parts": [{
                    "text": "{\"agent_id\":\"gemini\",\"verdict\":\"pass\",\"confidence\":0.90,\"issues\":[],\"suggestions\":[\"Good code structure\"],\"sign_off\":true,\"reasoning\":\"The code is well-written.\"}"
                }],
                "role": "model"
            },
            "finishReason": "STOP"
        }]
    }"#
    .to_string()
}

/// Generate a mock text response for Grok/Codex
#[cfg(not(feature = "http"))]
fn mock_text_response() -> String {
    r#"{
        "id": "mock-text-id",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "mock-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "This is a mock response from the AI agent."
            },
            "finish_reason": "stop"
        }]
    }"#
    .to_string()
}

/// Generate a mock text response for Claude
#[cfg(not(feature = "http"))]
fn mock_claude_text_response() -> String {
    r#"{
        "id": "msg_mock_text",
        "type": "message",
        "role": "assistant",
        "content": [
            {
                "type": "text",
                "text": "This is a mock response from Claude."
            }
        ],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 8
        }
    }"#
    .to_string()
}

/// Generate a mock text response for Gemini
#[cfg(not(feature = "http"))]
fn mock_gemini_text_response() -> String {
    r#"{
        "candidates": [{
            "content": {
                "parts": [{
                    "text": "This is a mock response from Gemini."
                }],
                "role": "model"
            },
            "finishReason": "STOP"
        }]
    }"#
    .to_string()
}

// ============================================================================
// Tests
// ============================================================================

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

    #[test]
    fn test_adapter_creation_error_display() {
        let err = AdapterCreationError::UnknownAgent("invalid".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unknown agent"));
        assert!(msg.contains("invalid"));
        assert!(msg.contains("grok"));
    }

    #[test]
    fn test_missing_credentials_error_display() {
        let err = AdapterCreationError::MissingCredentials("grok".to_string());
        let msg = err.to_string();
        assert!(msg.contains("No credentials found"));
        assert!(msg.contains("grok"));
        assert!(msg.contains("aiy credentials set xai"));
    }

    #[test]
    fn test_get_credential_providers() {
        assert_eq!(get_credential_providers("grok"), ("xai", Some("grok")));
        assert_eq!(
            get_credential_providers("claude"),
            ("anthropic", Some("claude"))
        );
        assert_eq!(
            get_credential_providers("gemini"),
            ("google", Some("gemini"))
        );
        assert_eq!(get_credential_providers("codex"), ("openai", Some("codex")));
        assert_eq!(get_credential_providers("unknown"), ("unknown", None));
    }

    #[test]
    fn test_valid_agents_list() {
        let list = valid_agents_list();
        assert!(list.contains("grok"));
        assert!(list.contains("claude"));
        assert!(list.contains("gemini"));
        assert!(list.contains("codex"));
    }

    #[tokio::test]
    async fn test_has_credentials_with_primary_key() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under primary provider
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("xai", "test-key").unwrap();
        }

        assert!(has_credentials("grok", &manager).await);
    }

    #[tokio::test]
    async fn test_has_credentials_with_fallback_key() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store key under fallback provider
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("grok", "test-key").unwrap();
        }

        assert!(has_credentials("grok", &manager).await);
    }

    #[tokio::test]
    async fn test_has_credentials_missing() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Don't store any keys
        assert!(!has_credentials("grok", &manager).await);
    }

    #[tokio::test]
    async fn test_get_available_agents_empty() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        let available = get_available_agents(&manager).await;
        assert!(available.is_empty());
    }

    #[tokio::test]
    async fn test_get_available_agents_with_credentials() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store keys for grok and claude
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("xai", "grok-key").unwrap();
            mgr.store_key("anthropic", "claude-key").unwrap();
        }

        let available = get_available_agents(&manager).await;
        assert_eq!(available.len(), 2);
        assert!(available.contains(&"grok"));
        assert!(available.contains(&"claude"));
    }

    #[tokio::test]
    async fn test_create_review_adapter_unknown_agent() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        let result = create_review_adapter("invalid", manager).await;
        assert!(result.is_err());
        if let Err(AdapterCreationError::UnknownAgent(id)) = result {
            assert_eq!(id, "invalid");
        } else {
            panic!("Expected UnknownAgent error");
        }
    }

    #[tokio::test]
    async fn test_create_ask_adapter_unknown_agent() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        let result = create_ask_adapter("invalid", manager).await;
        assert!(result.is_err());
        if let Err(AdapterCreationError::UnknownAgent(id)) = result {
            assert_eq!(id, "invalid");
        } else {
            panic!("Expected UnknownAgent error");
        }
    }

    #[tokio::test]
    async fn test_create_review_adapter_missing_credentials() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        let result = create_review_adapter("grok", manager).await;
        assert!(result.is_err());
        if let Err(AdapterCreationError::MissingCredentials(id)) = result {
            assert_eq!(id, "grok");
        } else {
            panic!("Expected MissingCredentials error");
        }
    }

    #[tokio::test]
    async fn test_ask_adapter_methods() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("xai", "test-key").unwrap();
        }

        // Note: This test only works in non-HTTP mode
        #[cfg(not(feature = "http"))]
        {
            let adapter = create_ask_adapter("grok", manager).await.unwrap();
            assert_eq!(adapter.agent_id(), "grok");
            assert_eq!(adapter.display_name(), "Grok (xAI)");
            assert_eq!(adapter.model_name(), "grok-4-1-fast");
        }
    }

    #[cfg(not(feature = "http"))]
    #[tokio::test]
    async fn test_create_all_adapters_with_mock() {
        let (manager, _temp) = setup_test_credential_manager();
        setup_and_unlock(&manager).await;

        // Store credentials for all agents
        {
            let mut mgr = manager.lock().await;
            mgr.store_key("xai", "grok-key").unwrap();
            mgr.store_key("anthropic", "claude-key").unwrap();
            mgr.store_key("google", "gemini-key").unwrap();
            mgr.store_key("openai", "codex-key").unwrap();
        }

        // Create review adapters
        for agent in ["grok", "claude", "gemini", "codex"] {
            let result = create_review_adapter(agent, manager.clone()).await;
            assert!(
                result.is_ok(),
                "Failed to create review adapter for {}",
                agent
            );
        }

        // Create ask adapters
        for agent in ["grok", "claude", "gemini", "codex"] {
            let result = create_ask_adapter(agent, manager.clone()).await;
            assert!(result.is_ok(), "Failed to create ask adapter for {}", agent);
        }
    }
}
