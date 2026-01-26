//! Credentials command implementation
//!
//! Manages API credentials using aiy-core's secure CredentialManager.

use crate::credential_helper::{self, CREDENTIALS_PASSWORD_ENV};
use crate::CredentialsCommands;
use aiy_core::security::CredentialManager;
use aiy_core::PipelineConfig;
use std::io::IsTerminal;

/// Get the credential manager with backend selection and unlock
fn get_credential_manager() -> anyhow::Result<(CredentialManager, bool)> {
    let config = PipelineConfig::load(&PipelineConfig::config_path()?)
        .unwrap_or_else(|_| PipelineConfig::default());

    credential_helper::create_credential_manager(&config)
}

/// Unlock manager using env var or prompt
fn unlock_manager(manager: &mut CredentialManager, needs_password: bool) -> anyhow::Result<()> {
    if !needs_password {
        return Ok(()); // System keychain doesn't need unlock
    }

    // Try environment variable first (enables non-interactive automation)
    if let Ok(password) = std::env::var(CREDENTIALS_PASSWORD_ENV) {
        manager.unlock(&password).map_err(|_| {
            anyhow::anyhow!(
                "Failed to unlock credentials with {}.\n\
                 Check that the password is correct.",
                CREDENTIALS_PASSWORD_ENV
            )
        })?;
        return Ok(());
    }

    // Non-interactive: require env var instead of attempting to prompt
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "Credentials are locked. Set {} environment variable\n\
             or run interactively to enter the password.",
            CREDENTIALS_PASSWORD_ENV
        );
    }

    // Fall back to interactive prompt
    let password = prompt_password("Enter master password: ")?;
    manager.unlock(&password)?;
    Ok(())
}

/// Prompt for master password securely (no echo)
fn prompt_password(prompt_text: &str) -> anyhow::Result<String> {
    let password = rpassword::prompt_password(prompt_text)?;
    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }
    Ok(password)
}

/// Prompt for API key securely (no echo)
fn prompt_api_key(provider: &str) -> anyhow::Result<String> {
    let api_key = rpassword::prompt_password(format!("Enter API key for '{}': ", provider))?;
    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }
    Ok(api_key)
}

/// Run a credentials subcommand
pub fn run(cmd: CredentialsCommands) -> anyhow::Result<()> {
    match cmd {
        CredentialsCommands::Status => {
            let (mut manager, needs_password) = get_credential_manager()?;
            unlock_manager(&mut manager, needs_password)?;

            let providers = manager.list_providers()?;
            let backend = manager.backend();

            println!("\nCredential Status:");
            println!("─────────────────");
            println!("Backend: {:?}", backend);
            println!("Stored providers: {}", providers.len());

            if providers.is_empty() {
                println!("  (no credentials stored)");
            } else {
                for provider in &providers {
                    println!("  - {}", provider);
                }
            }

            Ok(())
        }

        CredentialsCommands::Set { provider } => {
            let (mut manager, needs_password) = get_credential_manager()?;
            unlock_manager(&mut manager, needs_password)?;

            let api_key = prompt_api_key(&provider)?;
            manager.store_key(&provider, &api_key)?;

            println!("✓ Credentials stored for provider '{}'", provider);
            Ok(())
        }

        CredentialsCommands::Get { provider } => {
            let (mut manager, needs_password) = get_credential_manager()?;
            unlock_manager(&mut manager, needs_password)?;

            match manager.get_key(&provider) {
                Ok(key) => {
                    // NEVER print the full key - show redacted version only
                    let redacted = redact_api_key(&key);
                    println!("\nProvider: {}", provider);
                    println!("Key: {}", redacted);
                }
                Err(e) => {
                    eprintln!("Error retrieving credentials for '{}': {}", provider, e);
                    std::process::exit(1);
                }
            }

            Ok(())
        }

        CredentialsCommands::Delete { provider } => {
            let (mut manager, needs_password) = get_credential_manager()?;
            unlock_manager(&mut manager, needs_password)?;

            manager.delete_key(&provider)?;
            println!("✓ Credentials deleted for provider '{}'", provider);
            Ok(())
        }
    }
}

/// Redact an API key for safe display
///
/// Shows first 4 and last 4 characters only
fn redact_api_key(key: &str) -> String {
    if key.len() <= 12 {
        "[REDACTED]".to_string()
    } else {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_key() {
        assert_eq!(redact_api_key("short"), "[REDACTED]");
        assert_eq!(redact_api_key("sk-1234567890abcdefghij"), "sk-1...ghij");
        assert_eq!(redact_api_key("very-long-api-key-value"), "very...alue");
    }
}
