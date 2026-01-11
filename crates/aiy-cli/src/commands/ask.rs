//! Ask command implementation
//!
//! Provides the `aiy ask --agent <agent> --prompt "..."` command for querying
//! a single AI agent with a prompt and receiving a text response.

#[cfg(feature = "http")]
use aiy_adapter_grok::GrokClient;
use aiy_adapter_grok::{GrokAdapter, GrokError};
use aiy_core::security::{CredentialBackend, CredentialManager};
use chrono::Utc;
use serde::Serialize;
use std::io::{self, BufRead, IsTerminal};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Output format for ask results
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Plain text output (default)
    #[default]
    Text,
    /// JSON output for scripting
    Json,
}

/// Arguments for the ask command
#[derive(Debug)]
pub struct AskArgs {
    /// Agent to use (e.g., grok)
    pub agent: String,
    /// Prompt to send to the agent
    pub prompt: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// JSON output structure for ask results
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

/// Run the ask command
pub async fn run(args: AskArgs) -> anyhow::Result<()> {
    // Validate agent
    let agent = args.agent.to_lowercase();
    if agent != "grok" {
        anyhow::bail!(
            "Agent '{}' is not supported yet. Currently supported agents: grok\n\
             More agents coming soon!",
            args.agent
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

    // Get credential path
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let cred_path = config_dir.join("all-in-yum").join("credentials.enc");

    // Create credential manager
    let credential_manager = CredentialManager::new(CredentialBackend::EncryptedFile {
        path: cred_path.clone(),
    })?;
    let credential_manager = Arc::new(Mutex::new(credential_manager));

    // Unlock credential manager
    unlock_credentials(&credential_manager, &cred_path).await?;

    // Create the Grok adapter
    let adapter = create_grok_adapter(credential_manager).await?;

    // Generate response
    let response_text = adapter
        .generate_text(&prompt)
        .await
        .map_err(format_grok_error)?;

    // Output based on format
    match args.format {
        OutputFormat::Text => {
            println!("{}", response_text);
        }
        OutputFormat::Json => {
            let response = AskResponse {
                agent_id: "grok".to_string(),
                model: "grok-4-1-fast".to_string(),
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

/// Unlock the credential manager
async fn unlock_credentials(
    credential_manager: &Arc<Mutex<CredentialManager>>,
    cred_path: &std::path::Path,
) -> anyhow::Result<()> {
    // Check if credentials file exists
    if !cred_path.exists() {
        anyhow::bail!(
            "No credentials configured. Set up your API key with:\n  \
             aiy credentials set xai"
        );
    }

    // Try to unlock with environment variable or prompt
    let mut manager = credential_manager.lock().await;

    // First try environment variable
    if let Ok(password) = std::env::var("AIY_CREDENTIALS_PASSWORD") {
        manager.unlock(&password).map_err(|_| {
            anyhow::anyhow!(
                "Failed to unlock credentials with AIY_CREDENTIALS_PASSWORD.\n\
                 Check that the password is correct."
            )
        })?;
        return Ok(());
    }

    // For non-interactive use, require the environment variable
    if std::io::stdin().is_terminal() {
        // Interactive: prompt for password
        let password = rpassword::prompt_password("Enter credentials password: ")?;
        manager.unlock(&password).map_err(|_| {
            anyhow::anyhow!("Failed to unlock credentials. Incorrect password?")
        })?;
    } else {
        // Non-interactive: require AIY_CREDENTIALS_PASSWORD
        anyhow::bail!(
            "Credentials are locked. Set AIY_CREDENTIALS_PASSWORD environment variable\n\
             or run interactively to enter the password."
        );
    }

    Ok(())
}

/// Create a Grok adapter with HTTP transport (when available) or error
#[cfg(feature = "http")]
async fn create_grok_adapter(
    credential_manager: Arc<Mutex<CredentialManager>>,
) -> anyhow::Result<GrokAdapter> {
    let client = GrokClient::new_with_http(credential_manager)?;
    Ok(GrokAdapter::new(client))
}

#[cfg(not(feature = "http"))]
async fn create_grok_adapter(
    _credential_manager: Arc<Mutex<CredentialManager>>,
) -> anyhow::Result<GrokAdapter> {
    anyhow::bail!(
        "HTTP transport not enabled. Build with `--features http` for real API calls.\n\
         For testing, use the mock transport via the test harness."
    )
}

/// Format Grok error into a user-friendly message without leaking secrets
fn format_grok_error(error: GrokError) -> anyhow::Error {
    match error {
        GrokError::Credential(_) => {
            // Never print the actual credential error message as it might contain hints
            anyhow::anyhow!(
                "API key not found or invalid. Set up your xAI API key with:\n  \
                 aiy credentials set xai"
            )
        }
        GrokError::Transport(msg) => {
            // Sanitize transport errors
            let sanitized = sanitize_error_message(&msg);
            anyhow::anyhow!("Network error: {}", sanitized)
        }
        GrokError::ApiRequest(_) => {
            anyhow::anyhow!("API request failed. Check your network connection and API key.")
        }
        GrokError::ResponseParsing(msg) => {
            anyhow::anyhow!("Failed to parse response from Grok API: {}", msg)
        }
        GrokError::Serialization(_) => {
            anyhow::anyhow!("Failed to serialize request")
        }
        GrokError::SecurityValidation(msg) => {
            anyhow::anyhow!("Security check failed: {}", msg)
        }
        GrokError::SchemaValidation(msg) => {
            anyhow::anyhow!("Response validation failed: {}", msg)
        }
        GrokError::Other(msg) => {
            anyhow::anyhow!("Error: {}", msg)
        }
        GrokError::RateLimit(msg) => {
            anyhow::anyhow!("Rate limit exceeded: {}. Please try again later.", msg)
        }
        GrokError::Timeout(msg) => {
            anyhow::anyhow!("Request timed out: {}. Try again or check your connection.", msg)
        }
    }
}

/// Sanitize error messages to prevent secret leakage
fn sanitize_error_message(msg: &str) -> String {
    // Remove anything that looks like an API key or token
    let mut sanitized = msg.to_string();

    // Remove Bearer tokens
    if let Some(start) = sanitized.find("Bearer ") {
        let end = sanitized[start..]
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
            .map(|i| start + i)
            .unwrap_or(sanitized.len());
        sanitized.replace_range(start..end, "Bearer [REDACTED]");
    }

    // Remove anything that looks like an API key (long alphanumeric strings)
    let key_pattern = regex::Regex::new(r"[a-zA-Z0-9_-]{32,}").unwrap();
    sanitized = key_pattern.replace_all(&sanitized, "[REDACTED]").to_string();

    sanitized
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
    fn test_sanitize_error_message_bearer() {
        let msg = "Authorization: Bearer sk-xai-abc123def456ghi789jkl012mno345pqr678";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("sk-xai-abc123def456ghi789jkl012mno345pqr678"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_sanitize_error_message_long_key() {
        let msg = "API key xai_abcdefghijklmnopqrstuvwxyz123456 is invalid";
        let sanitized = sanitize_error_message(msg);
        assert!(!sanitized.contains("xai_abcdefghijklmnopqrstuvwxyz123456"));
        assert!(sanitized.contains("[REDACTED]"));
    }

    #[test]
    fn test_sanitize_error_message_no_secrets() {
        let msg = "Connection timed out after 30 seconds";
        let sanitized = sanitize_error_message(msg);
        assert_eq!(sanitized, msg);
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
    fn test_format_grok_error_credential() {
        let error = GrokError::Credential("secret key".to_string());
        let formatted = format_grok_error(error);
        let msg = formatted.to_string();
        assert!(!msg.contains("secret"));
        assert!(msg.contains("aiy credentials set xai"));
    }

    #[test]
    fn test_format_grok_error_transport() {
        let error = GrokError::Transport("connection failed".to_string());
        let formatted = format_grok_error(error);
        let msg = formatted.to_string();
        assert!(msg.contains("Network error"));
    }
}
