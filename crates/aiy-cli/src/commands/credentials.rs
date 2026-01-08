//! Credentials command implementation
//!
//! Manages API credentials using aiy-core's secure CredentialManager.

use crate::CredentialsCommands;
use aiy_core::security::{CredentialBackend, CredentialManager};
use aiy_core::PipelineConfig;
use std::path::PathBuf;

/// Get the credential manager with backend selection
///
/// Tries SystemKeychain first, falls back to EncryptedFile if unavailable
fn get_credential_manager() -> anyhow::Result<(CredentialManager, bool)> {
    // Load config to check backend preference
    let config = PipelineConfig::load(&PipelineConfig::config_path()?)
        .unwrap_or_else(|_| PipelineConfig::default());

    // Try system keychain first if preferred or as fallback
    if config.credential_backend == "system" {
        match CredentialManager::new(CredentialBackend::SystemKeychain) {
            Ok(manager) => {
                println!("Using system keychain for credentials");
                return Ok((manager, false)); // No password needed for keychain
            }
            Err(e) => {
                eprintln!("System keychain unavailable: {}", e);
                eprintln!("Falling back to encrypted file...");
            }
        }
    }

    // Use encrypted file backend
    let cred_path = get_credential_file_path()?;
    let manager = CredentialManager::new(CredentialBackend::EncryptedFile {
        path: cred_path.clone(),
    })?;

    println!("Using encrypted file: {}", cred_path.display());
    Ok((manager, true)) // Password needed for encrypted file
}

/// Get the default credential file path
fn get_credential_file_path() -> anyhow::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let aiy_dir = config_dir.join("all-in-yum");
    std::fs::create_dir_all(&aiy_dir)?;
    Ok(aiy_dir.join("credentials.enc"))
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

            if needs_password {
                let password = prompt_password("Enter master password: ")?;
                manager.unlock(&password)?;
            }

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

            if needs_password {
                let password = prompt_password("Enter master password: ")?;
                manager.unlock(&password)?;
            }

            let api_key = prompt_api_key(&provider)?;
            manager.store_key(&provider, &api_key)?;

            println!("✓ Credentials stored for provider '{}'", provider);
            Ok(())
        }

        CredentialsCommands::Get { provider } => {
            let (mut manager, needs_password) = get_credential_manager()?;

            if needs_password {
                let password = prompt_password("Enter master password: ")?;
                manager.unlock(&password)?;
            }

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

            if needs_password {
                let password = prompt_password("Enter master password: ")?;
                manager.unlock(&password)?;
            }

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
        assert_eq!(
            redact_api_key("sk-1234567890abcdefghij"),
            "sk-1...ghij"
        );
        assert_eq!(
            redact_api_key("very-long-api-key-value"),
            "very...alue"
        );
    }
}
