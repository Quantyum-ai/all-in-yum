//! Agent management commands
//!
//! Provides commands for listing, enabling, and disabling AI agents.

use crate::registry::{self, AGENTS};
use aiy_core::security::{CredentialBackend, CredentialManager};
use aiy_core::PipelineConfig;
use colored::*;

/// Agent status information (runtime status, not static metadata)
#[derive(Debug)]
struct AgentStatus {
    id: String,
    display_name: String,
    enabled: bool,
    has_credentials: bool,
    credential_provider: String,
}

/// Get all known agents with their runtime status
fn get_agent_status(config: &PipelineConfig, manager: Option<&CredentialManager>) -> Vec<AgentStatus> {
    AGENTS
        .iter()
        .map(|agent| {
            let enabled = config.enabled_agents.contains(&agent.id.to_string());
            let has_credentials = manager
                .map(|m| {
                    m.get_key(agent.credential_provider).is_ok()
                        || m.get_key(agent.id).is_ok() // Check fallback
                })
                .unwrap_or(false);

            AgentStatus {
                id: agent.id.to_string(),
                display_name: agent.display_name.to_string(),
                enabled,
                has_credentials,
                credential_provider: agent.credential_provider.to_string(),
            }
        })
        .collect()
}

/// List available agents with their status
pub fn list() -> anyhow::Result<()> {
    let config = PipelineConfig::load(&PipelineConfig::config_path()?)
        .unwrap_or_else(|_| PipelineConfig::default());

    let manager = get_credential_manager(&config).ok();
    let agents = get_agent_status(&config, manager.as_ref());

    println!("\n{}", "Available Agents".cyan().bold());
    println!("{}", "================".cyan());
    println!();

    // Header
    println!(
        "{:12} {:25} {:10} {:12}",
        "ID".white().bold(),
        "Name".white().bold(),
        "Status".white().bold(),
        "Credentials".white().bold()
    );
    println!("{}", "-".repeat(60));

    for agent in &agents {
        let status = if agent.enabled {
            "enabled".green()
        } else {
            "disabled".red()
        };

        let creds = if agent.has_credentials {
            "configured".green()
        } else {
            "missing".yellow()
        };

        println!(
            "{:12} {:25} {:10} {:12}",
            agent.id,
            agent.display_name,
            status,
            creds
        );
    }

    println!();
    println!(
        "{}: To enable an agent:  {}",
        "Tip".cyan(),
        "aiy agents enable <id>".white()
    );
    println!(
        "     To set credentials: {}",
        "aiy credentials set <provider>".white()
    );
    println!();

    Ok(())
}

/// Enable an agent
pub fn enable(name: String) -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    let mut config = PipelineConfig::load(&config_path).unwrap_or_else(|_| PipelineConfig::default());

    // Validate agent name using registry
    let agent = registry::get_agent(&name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown agent '{}'. Valid agents are: {}",
            name,
            registry::valid_agents_string()
        )
    })?;

    if config.enabled_agents.contains(&name) {
        println!("{} Agent '{}' is already enabled", "[Info]".cyan(), name);
        return Ok(());
    }

    config.enabled_agents.push(name.clone());
    config.save(&config_path)?;

    println!(
        "{} Agent '{}' has been enabled",
        "[OK]".green(),
        name.white().bold()
    );

    // Show credential hint using registry metadata
    println!(
        "\n{}: Set credentials with: {}",
        "Note".cyan(),
        format!("aiy credentials set {}", agent.credential_provider).white()
    );

    Ok(())
}

/// Disable an agent
pub fn disable(name: String) -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    let mut config = PipelineConfig::load(&config_path).unwrap_or_else(|_| PipelineConfig::default());

    // Validate agent name using registry
    if !registry::is_valid_agent(&name) {
        anyhow::bail!(
            "Unknown agent '{}'. Valid agents are: {}",
            name,
            registry::valid_agents_string()
        );
    }

    if !config.enabled_agents.contains(&name) {
        println!("{} Agent '{}' is already disabled", "[Info]".cyan(), name);
        return Ok(());
    }

    config.enabled_agents.retain(|a| a != &name);
    config.save(&config_path)?;

    println!(
        "{} Agent '{}' has been disabled",
        "[OK]".green(),
        name.white().bold()
    );

    Ok(())
}

/// Show which agents have valid credentials
pub fn status() -> anyhow::Result<()> {
    let config = PipelineConfig::load(&PipelineConfig::config_path()?)
        .unwrap_or_else(|_| PipelineConfig::default());

    let manager = get_credential_manager(&config)?;
    let agents = get_agent_status(&config, Some(&manager));

    println!("\n{}", "Agent Credential Status".cyan().bold());
    println!("{}", "=======================".cyan());
    println!();

    let mut ready_count = 0;
    let mut enabled_count = 0;

    for agent in &agents {
        if agent.enabled {
            enabled_count += 1;

            let status_icon = if agent.has_credentials {
                ready_count += 1;
                "[OK]".green()
            } else {
                "[!]".yellow()
            };

            let status_text = if agent.has_credentials {
                "Ready".green()
            } else {
                "Missing credentials".yellow()
            };

            println!(
                "{} {} - {}",
                status_icon,
                agent.display_name.white().bold(),
                status_text
            );

            if !agent.has_credentials {
                println!(
                    "    Set with: {}",
                    format!("aiy credentials set {}", agent.credential_provider).dimmed()
                );
            }
        }
    }

    println!();
    println!(
        "{} {} of {} enabled agents ready",
        if ready_count == enabled_count {
            "[OK]".green()
        } else {
            "[!]".yellow()
        },
        ready_count,
        enabled_count
    );

    if ready_count == 0 {
        println!();
        println!(
            "{}: No agents are ready. Set up credentials first:",
            "Note".yellow()
        );
        // Generate hints from registry
        for agent in AGENTS {
            println!(
                "  aiy credentials set {:10} # for {}",
                agent.credential_provider, agent.display_name
            );
        }
    }

    Ok(())
}

/// Get credential manager based on config
fn get_credential_manager(config: &PipelineConfig) -> anyhow::Result<CredentialManager> {
    // Try system keychain first if preferred
    if config.credential_backend == "system" {
        if let Ok(manager) = CredentialManager::new(CredentialBackend::SystemKeychain) {
            return Ok(manager);
        }
    }

    // Fallback to encrypted file
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let cred_path = config_dir.join("all-in-yum").join("credentials.enc");

    let manager = CredentialManager::new(CredentialBackend::EncryptedFile {
        path: cred_path,
    })?;

    Ok(manager)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_agent_status_no_manager() {
        let config = PipelineConfig::default();
        let agents = get_agent_status(&config, None);

        // Agent count should match registry
        assert_eq!(agents.len(), AGENTS.len());

        // All registry agents should be present
        for reg_agent in AGENTS {
            assert!(agents.iter().any(|a| a.id == reg_agent.id));
        }

        // All should have no credentials without a manager
        for agent in &agents {
            assert!(!agent.has_credentials);
        }
    }

    #[test]
    fn test_agent_enabled_status() {
        let config = PipelineConfig {
            enabled_agents: vec!["grok".to_string(), "claude".to_string()],
            ..Default::default()
        };

        let agents = get_agent_status(&config, None);

        let grok = agents.iter().find(|a| a.id == "grok").unwrap();
        assert!(grok.enabled);

        let gemini = agents.iter().find(|a| a.id == "gemini").unwrap();
        assert!(!gemini.enabled);
    }
}
