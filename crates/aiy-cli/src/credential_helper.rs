//! Unified credential backend selection and unlock helper
//!
//! This module provides consistent credential management across all CLI commands.

use aiy_core::security::{CredentialBackend, CredentialManager};
use aiy_core::PipelineConfig;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Environment variable for non-interactive credential unlock
pub const CREDENTIALS_PASSWORD_ENV: &str = "AIY_CREDENTIALS_PASSWORD";

/// Get credential file path
pub fn get_credential_path() -> anyhow::Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    Ok(config_dir.join("all-in-yum").join("credentials.enc"))
}

/// Create credential manager based on config preference
/// Returns (manager, needs_unlock) tuple
pub fn create_credential_manager(
    config: &PipelineConfig,
) -> anyhow::Result<(CredentialManager, bool)> {
    // Try system keychain first if preferred
    if config.credential_backend == "system" {
        if let Ok(manager) = CredentialManager::new(CredentialBackend::SystemKeychain) {
            return Ok((manager, false)); // System keychain doesn't need unlock
        }
        // Fall through to encrypted file
    }

    // Use encrypted file backend
    let cred_path = get_credential_path()?;
    let manager = CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path })?;
    Ok((manager, true)) // File backend needs unlock
}

/// Unlock credential manager using env var or interactive prompt
///
/// Returns error if:
/// - Non-interactive mode and no AIY_CREDENTIALS_PASSWORD set
/// - Password is incorrect
pub fn unlock_credential_manager(
    manager: &mut CredentialManager,
    cred_path: &std::path::Path,
) -> anyhow::Result<()> {
    // Check if credentials file exists
    if !cred_path.exists() {
        anyhow::bail!(
            "No credentials configured. Set up your API key with:\n  \
             aiy credentials set <provider>\n\n\
             Providers: xai (Grok), anthropic (Claude), google (Gemini), openai (Codex)"
        );
    }

    // First try environment variable
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

    // For non-interactive use, require the environment variable
    if std::io::stdin().is_terminal() {
        // Interactive: prompt for password
        let password = rpassword::prompt_password("Enter credentials password: ")?;
        manager
            .unlock(&password)
            .map_err(|_| anyhow::anyhow!("Failed to unlock credentials. Incorrect password?"))?;
    } else {
        // Non-interactive: require AIY_CREDENTIALS_PASSWORD
        anyhow::bail!(
            "Credentials are locked. Set {} environment variable\n\
             or run interactively to enter the password.",
            CREDENTIALS_PASSWORD_ENV
        );
    }

    Ok(())
}

/// Create and unlock credential manager (convenience function)
pub async fn get_unlocked_credential_manager(
    config: &PipelineConfig,
) -> anyhow::Result<Arc<Mutex<CredentialManager>>> {
    let (mut manager, needs_unlock) = create_credential_manager(config)?;

    if needs_unlock {
        let cred_path = get_credential_path()?;
        unlock_credential_manager(&mut manager, &cred_path)?;
    }

    Ok(Arc::new(Mutex::new(manager)))
}
