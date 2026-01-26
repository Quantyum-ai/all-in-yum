//! Ask command implementation
//!
//! Provides the `aiy ask --agent <agent> --prompt "..."` command for querying
//! a single AI agent with a prompt and receiving a text response.

use crate::adapters::{create_ask_adapter, AdapterCreationError};
use crate::credential_helper;
use crate::registry::{get_agent, is_valid_agent, valid_agents_string};
use aiy_core::PipelineConfig;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::io::{self, BufRead, IsTerminal};
use std::time::Duration;

/// Output format for ask results
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Plain text output (default)
    #[default]
    Text,
    /// JSON output for scripting
    Json,
}

/// Arguments for the ask command.
///
/// Contains all the parameters needed to execute an ask operation against
/// a single AI agent.
#[derive(Debug)]
pub struct AskArgs {
    /// Agent to use (e.g., grok, claude, gemini, codex)
    pub agent: String,
    /// Prompt to send to the agent
    pub prompt: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// JSON output structure for ask results.
///
/// This struct is serialized when the user requests JSON output format,
/// providing machine-readable response data for scripting and automation.
#[derive(Debug, Serialize)]
pub struct AskResponse {
    /// Agent identifier
    pub agent_id: String,
    /// Model used
    pub model: String,
    /// Response text
    pub text: String,
    /// Timestamp of response
    pub timestamp: String,
}

/// Run the ask command.
///
/// Sends a prompt to the specified AI agent and outputs the response.
/// Supports both interactive terminal prompts and piped stdin input.
///
/// # Arguments
///
/// * `args` - The ask command arguments including agent, prompt, and format
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the operation fails.
///
/// # Errors
///
/// This function will return an error if:
/// - The specified agent is not supported
/// - The prompt is empty or missing
/// - Credentials are not configured or invalid
/// - The API request fails
pub async fn run(args: AskArgs) -> anyhow::Result<()> {
    // Validate agent using registry
    let agent = args.agent.to_lowercase();
    if !is_valid_agent(&agent) {
        anyhow::bail!(
            "Unknown agent '{}'. Valid agents: {}\n\
             Use 'aiy agents list' to see available agents.",
            args.agent,
            valid_agents_string()
        );
    }

    // Get prompt from args or stdin
    let prompt = match args.prompt {
        Some(p) => p,
        None => read_prompt_from_stdin()?,
    };

    if prompt.trim().is_empty() {
        anyhow::bail!("Prompt cannot be empty. Provide a prompt with --prompt or via stdin.");
    }

    // Load config and get unlocked credential manager
    let config = PipelineConfig::load(&PipelineConfig::config_path()?)
        .unwrap_or_else(|_| PipelineConfig::default());

    let credential_manager = credential_helper::get_unlocked_credential_manager(&config).await?;

    // Create adapter using factory
    let adapter = create_ask_adapter(&agent, credential_manager)
        .await
        .map_err(format_adapter_error)?;

    // Get agent display name from registry
    let agent_display_name = get_agent(&agent).map(|a| a.display_name).unwrap_or(&agent);

    // Create progress spinner for ask operation
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid spinner template"),
    );
    pb.set_message(format!("Asking {}...", agent_display_name));
    pb.enable_steady_tick(Duration::from_millis(100));

    // Generate response
    let response_text = adapter
        .generate_text(&prompt)
        .await
        .map_err(format_adapter_error)?;

    pb.finish_and_clear();

    // Output based on format
    match args.format {
        OutputFormat::Text => {
            println!("{}", response_text);
        }
        OutputFormat::Json => {
            let response = AskResponse {
                agent_id: adapter.agent_id().to_string(),
                model: adapter.model_name().to_string(),
                text: response_text,
                timestamp: Utc::now().to_rfc3339(),
            };
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}

/// Read prompt from stdin (for piping support)
fn read_prompt_from_stdin() -> anyhow::Result<String> {
    let stdin = io::stdin();

    // Check if stdin is a terminal (interactive) or has piped input
    if stdin.is_terminal() {
        anyhow::bail!(
            "No prompt provided. Use --prompt/-p or pipe input:\n  \
             aiy ask --agent grok --prompt \"your question\"\n  \
             echo \"your question\" | aiy ask --agent grok"
        );
    }

    // Read all lines from stdin
    let mut prompt = String::new();
    for line in stdin.lock().lines() {
        let line = line?;
        if !prompt.is_empty() {
            prompt.push('\n');
        }
        prompt.push_str(&line);
    }

    Ok(prompt)
}

/// Format adapter creation error into a user-friendly message without leaking secrets
fn format_adapter_error(error: AdapterCreationError) -> anyhow::Error {
    match error {
        AdapterCreationError::UnknownAgent(id) => {
            anyhow::anyhow!(
                "Unknown agent '{}'. Valid agents: {}\n\
                 Use 'aiy agents list' to see available agents.",
                id,
                valid_agents_string()
            )
        }
        AdapterCreationError::MissingCredentials(id) => {
            let provider = get_credential_provider(&id);
            anyhow::anyhow!(
                "No credentials found for '{}'. Set up your API key with:\n  \
                 aiy credentials set {}",
                id,
                provider
            )
        }
        AdapterCreationError::CreationFailed(msg) => {
            anyhow::anyhow!("Failed to create adapter: {}", msg)
        }
        AdapterCreationError::HttpNotEnabled(id) => {
            anyhow::anyhow!(
                "HTTP transport not enabled for '{}'. Build with `--features http` for real API calls.",
                id
            )
        }
        AdapterCreationError::Grok(e) => {
            anyhow::anyhow!("Grok error: {}", e.to_sanitized_string())
        }
        AdapterCreationError::Claude(e) => {
            anyhow::anyhow!("Claude error: {}", e.to_sanitized_string())
        }
        AdapterCreationError::Gemini(e) => {
            anyhow::anyhow!("Gemini error: {}", e.to_sanitized_string())
        }
        AdapterCreationError::Codex(e) => {
            anyhow::anyhow!("Codex error: {}", e.to_sanitized_string())
        }
    }
}

/// Get the credential provider for an agent
fn get_credential_provider(agent_id: &str) -> &'static str {
    match agent_id {
        "grok" => "xai",
        "claude" => "anthropic",
        "gemini" => "google",
        "codex" => "openai",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        let format = OutputFormat::default();
        assert!(matches!(format, OutputFormat::Text));
    }

    #[test]
    fn test_ask_response_serialization() {
        let response = AskResponse {
            agent_id: "grok".to_string(),
            model: "grok-4-1-fast".to_string(),
            text: "Hello, world!".to_string(),
            timestamp: "2026-01-11T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"agent_id\":\"grok\""));
        assert!(json.contains("\"model\":\"grok-4-1-fast\""));
        assert!(json.contains("\"text\":\"Hello, world!\""));
    }

    #[test]
    fn test_ask_args_creation() {
        let args = AskArgs {
            agent: "grok".to_string(),
            prompt: Some("Hello".to_string()),
            format: OutputFormat::Text,
        };

        assert_eq!(args.agent, "grok");
        assert_eq!(args.prompt, Some("Hello".to_string()));
    }

    #[test]
    fn test_get_credential_provider() {
        assert_eq!(get_credential_provider("grok"), "xai");
        assert_eq!(get_credential_provider("claude"), "anthropic");
        assert_eq!(get_credential_provider("gemini"), "google");
        assert_eq!(get_credential_provider("codex"), "openai");
        assert_eq!(get_credential_provider("unknown"), "unknown");
    }

    #[test]
    fn test_format_adapter_error_unknown_agent() {
        let error = AdapterCreationError::UnknownAgent("invalid".to_string());
        let formatted = format_adapter_error(error);
        let msg = formatted.to_string();
        assert!(msg.contains("Unknown agent"));
        assert!(msg.contains("invalid"));
    }

    #[test]
    fn test_format_adapter_error_missing_credentials() {
        let error = AdapterCreationError::MissingCredentials("grok".to_string());
        let formatted = format_adapter_error(error);
        let msg = formatted.to_string();
        assert!(msg.contains("No credentials found"));
        assert!(msg.contains("aiy credentials set xai"));
    }
}
