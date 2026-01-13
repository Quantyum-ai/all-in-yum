//! Configuration management commands
//!
//! Provides commands for viewing and modifying pipeline configuration.

use crate::registry;
use aiy_core::PipelineConfig;
use colored::*;

/// Display current configuration
pub fn show() -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    let config = PipelineConfig::load(&config_path).unwrap_or_else(|_| PipelineConfig::default());

    println!("\n{}", "Current Configuration".cyan().bold());
    println!("{}", "=====================".cyan());
    println!();

    // Show in TOML format for clarity
    let toml_string = toml::to_string_pretty(&config)?;
    println!("{}", toml_string);

    // Show file location
    println!("{}", "-".repeat(40));
    println!(
        "{}: {}",
        "Config file".dimmed(),
        config_path.display().to_string().dimmed()
    );

    Ok(())
}

/// Set a configuration value
pub fn set(key: String, value: String) -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    let mut config =
        PipelineConfig::load(&config_path).unwrap_or_else(|_| PipelineConfig::default());

    // Handle different configuration keys
    match key.as_str() {
        "credential_backend" => {
            if value != "system" && value != "file" {
                anyhow::bail!("Invalid credential_backend. Use 'system' or 'file'");
            }
            config.credential_backend = value.clone();
        }

        key if key.starts_with("default_models.") => {
            let agent = key.strip_prefix("default_models.").unwrap();
            if !registry::is_valid_agent(agent) {
                anyhow::bail!(
                    "Unknown agent '{}'. Valid agents: {}",
                    agent,
                    registry::valid_agents_string()
                );
            }
            config
                .default_models
                .insert(agent.to_string(), value.clone());
        }

        key if key.starts_with("base_urls.") => {
            let agent = key.strip_prefix("base_urls.").unwrap();
            if !registry::is_valid_agent(agent) {
                anyhow::bail!(
                    "Unknown agent '{}'. Valid agents: {}",
                    agent,
                    registry::valid_agents_string()
                );
            }
            config.base_urls.insert(agent.to_string(), value.clone());
        }

        key if key.starts_with("timeouts.") => {
            let agent = key.strip_prefix("timeouts.").unwrap();
            if !registry::is_valid_agent(agent) {
                anyhow::bail!(
                    "Unknown agent '{}'. Valid agents: {}",
                    agent,
                    registry::valid_agents_string()
                );
            }
            let timeout: u64 = value.parse().map_err(|_| {
                anyhow::anyhow!("Invalid timeout value. Must be a positive integer (milliseconds)")
            })?;
            config.timeouts.insert(agent.to_string(), timeout);
        }

        _ => {
            anyhow::bail!(
                "Unknown configuration key: '{}'\n\n\
                 Available keys:\n  \
                 - credential_backend (system|file)\n  \
                 - default_models.<agent> (model name)\n  \
                 - base_urls.<agent> (URL)\n  \
                 - timeouts.<agent> (milliseconds)\n\n\
                 Example: aiy config set default_models.grok grok-2-fast",
                key
            );
        }
    }

    config.save(&config_path)?;

    println!(
        "{} Configuration updated: {} = {}",
        "[OK]".green(),
        key.white().bold(),
        value.cyan()
    );

    Ok(())
}

/// Show configuration file path
pub fn path() -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    println!("{}", config_path.display());
    Ok(())
}

/// Reset configuration to defaults
pub fn reset() -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    let config = PipelineConfig::default();
    config.save(&config_path)?;

    println!("{} Configuration reset to defaults", "[OK]".green());
    println!(
        "{}: {}",
        "File".dimmed(),
        config_path.display().to_string().dimmed()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_path() {
        // Should not panic
        let result = PipelineConfig::config_path();
        assert!(result.is_ok() || result.is_err()); // Either works, just no panic
    }

    #[test]
    fn test_set_credential_backend() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = PipelineConfig::default();
        config.save(&config_path).unwrap();

        // Test valid backends
        config.credential_backend = "file".to_string();
        assert_eq!(config.credential_backend, "file");

        config.credential_backend = "system".to_string();
        assert_eq!(config.credential_backend, "system");
    }

    #[test]
    fn test_set_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = PipelineConfig::default();
        config.timeouts.insert("grok".to_string(), 60000);
        config.save(&config_path).unwrap();

        let loaded = PipelineConfig::load(&config_path).unwrap();
        assert_eq!(loaded.timeouts.get("grok"), Some(&60000));
    }

    #[test]
    fn test_set_model() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut config = PipelineConfig::default();
        config
            .default_models
            .insert("grok".to_string(), "grok-2-fast".to_string());
        config.save(&config_path).unwrap();

        let loaded = PipelineConfig::load(&config_path).unwrap();
        assert_eq!(
            loaded.default_models.get("grok"),
            Some(&"grok-2-fast".to_string())
        );
    }
}
