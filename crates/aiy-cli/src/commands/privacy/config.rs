//! Privacy configuration subcommands
//!
//! Provides commands for viewing and modifying privacy-specific configuration.

use aiy_core::config::{PipelineConfig, PrivacyModeConfig};
use colored::*;
use std::path::Path;

/// Show privacy configuration
pub fn show_config(repo_root: Option<&Path>) -> anyhow::Result<()> {
    let config = PipelineConfig::load_effective(repo_root)?;
    let privacy = config.get_privacy_mode();

    println!("\n{}", "Privacy Mode Configuration".cyan().bold());
    println!("{}", "==========================".cyan());
    println!();

    // Serialize to TOML for display
    let privacy_toml = toml::to_string_pretty(&privacy)?;
    println!("{}", privacy_toml);

    Ok(())
}

/// Set a privacy configuration value
pub fn set_config(repo_root: Option<&Path>, key: &str, value: &str) -> anyhow::Result<()> {
    // Determine config path
    let (config_path, mut config) = if let Some(root) = repo_root {
        let project_config = root.join(".aiy").join("config.toml");
        let config = if project_config.exists() {
            PipelineConfig::load(&project_config)?
        } else {
            PipelineConfig::default()
        };
        (project_config, config)
    } else {
        let global_path = PipelineConfig::config_path()?;
        let config = if global_path.exists() {
            PipelineConfig::load(&global_path)?
        } else {
            PipelineConfig::default()
        };
        (global_path, config)
    };

    // Ensure privacy mode config exists
    if config.privacy_mode.is_none() {
        config.privacy_mode = Some(PrivacyModeConfig::default());
    }

    let privacy = config.privacy_mode.as_mut().unwrap();

    // Parse the key and set value
    match key {
        "enabled" => {
            let enabled = parse_bool(value)?;
            privacy.enabled = enabled;
        }

        // Local executor settings
        "local_executor.ollama_url" | "ollama_url" => {
            privacy.local_executor.ollama_url = value.to_string();
            // Validate if privacy mode is enabled
            if privacy.enabled {
                privacy.local_executor.validate_loopback_only()?;
            }
        }
        "local_executor.model" | "model" => {
            privacy.local_executor.model = value.to_string();
        }
        "local_executor.context_size" | "context_size" => {
            privacy.local_executor.context_size = value
                .parse()
                .map_err(|_| anyhow::anyhow!("context_size must be a positive integer"))?;
        }
        "local_executor.timeout_ms" | "timeout_ms" => {
            privacy.local_executor.timeout_ms = value
                .parse()
                .map_err(|_| anyhow::anyhow!("timeout_ms must be a positive integer"))?;
        }
        "local_executor.temperature" | "temperature" => {
            let temp: f32 = value
                .parse()
                .map_err(|_| anyhow::anyhow!("temperature must be a float"))?;
            if !(0.0..=2.0).contains(&temp) {
                anyhow::bail!("temperature must be between 0.0 and 2.0");
            }
            privacy.local_executor.temperature = temp;
        }

        // RAG settings
        "rag.token_budget" | "token_budget" => {
            privacy.rag.token_budget = value
                .parse()
                .map_err(|_| anyhow::anyhow!("token_budget must be a positive integer"))?;
        }
        "rag.top_k" | "top_k" => {
            privacy.rag.top_k = value
                .parse()
                .map_err(|_| anyhow::anyhow!("top_k must be a positive integer"))?;
        }
        "rag.min_similarity" | "min_similarity" => {
            let sim: f32 = value
                .parse()
                .map_err(|_| anyhow::anyhow!("min_similarity must be a float"))?;
            if !(0.0..=1.0).contains(&sim) {
                anyhow::bail!("min_similarity must be between 0.0 and 1.0");
            }
            privacy.rag.min_similarity = sim;
        }

        // Verification settings
        "verification.max_fmt_repairs" | "max_fmt_repairs" => {
            privacy.verification.max_fmt_repairs = value
                .parse()
                .map_err(|_| anyhow::anyhow!("max_fmt_repairs must be a positive integer"))?;
        }
        "verification.max_clippy_repairs" | "max_clippy_repairs" => {
            privacy.verification.max_clippy_repairs = value
                .parse()
                .map_err(|_| anyhow::anyhow!("max_clippy_repairs must be a positive integer"))?;
        }
        "verification.max_test_repairs" | "max_test_repairs" => {
            privacy.verification.max_test_repairs = value
                .parse()
                .map_err(|_| anyhow::anyhow!("max_test_repairs must be a positive integer"))?;
        }
        "verification.max_global_repairs" | "max_global_repairs" => {
            privacy.verification.max_global_repairs = value
                .parse()
                .map_err(|_| anyhow::anyhow!("max_global_repairs must be a positive integer"))?;
        }

        _ => {
            anyhow::bail!(
                "Unknown privacy configuration key: '{}'\n\n\
                 Available keys:\n  \
                 - enabled (true|false)\n  \
                 - ollama_url (URL)\n  \
                 - model (model name)\n  \
                 - context_size (integer)\n  \
                 - timeout_ms (integer)\n  \
                 - temperature (float 0.0-2.0)\n  \
                 - token_budget (integer)\n  \
                 - top_k (integer)\n  \
                 - min_similarity (float 0.0-1.0)\n  \
                 - max_fmt_repairs (integer)\n  \
                 - max_clippy_repairs (integer)\n  \
                 - max_test_repairs (integer)\n  \
                 - max_global_repairs (integer)\n\n\
                 Example: aiy privacy config set model codellama:13b-instruct",
                key
            );
        }
    }

    config.save(&config_path)?;

    println!(
        "{} Privacy config updated: {} = {}",
        "[OK]".green(),
        key.white().bold(),
        value.cyan()
    );

    Ok(())
}

/// Get a privacy configuration value
pub fn get_config(repo_root: Option<&Path>, key: &str) -> anyhow::Result<()> {
    let config = PipelineConfig::load_effective(repo_root)?;
    let privacy = config.get_privacy_mode();

    let value = match key {
        "enabled" => privacy.enabled.to_string(),
        "local_executor.ollama_url" | "ollama_url" => privacy.local_executor.ollama_url.clone(),
        "local_executor.model" | "model" => privacy.local_executor.model.clone(),
        "local_executor.context_size" | "context_size" => {
            privacy.local_executor.context_size.to_string()
        }
        "local_executor.timeout_ms" | "timeout_ms" => {
            privacy.local_executor.timeout_ms.to_string()
        }
        "local_executor.temperature" | "temperature" => {
            privacy.local_executor.temperature.to_string()
        }
        "rag.token_budget" | "token_budget" => privacy.rag.token_budget.to_string(),
        "rag.top_k" | "top_k" => privacy.rag.top_k.to_string(),
        "rag.min_similarity" | "min_similarity" => privacy.rag.min_similarity.to_string(),
        "verification.max_fmt_repairs" | "max_fmt_repairs" => {
            privacy.verification.max_fmt_repairs.to_string()
        }
        "verification.max_clippy_repairs" | "max_clippy_repairs" => {
            privacy.verification.max_clippy_repairs.to_string()
        }
        "verification.max_test_repairs" | "max_test_repairs" => {
            privacy.verification.max_test_repairs.to_string()
        }
        "verification.max_global_repairs" | "max_global_repairs" => {
            privacy.verification.max_global_repairs.to_string()
        }
        _ => {
            anyhow::bail!("Unknown privacy configuration key: '{}'", key);
        }
    };

    println!("{}", value);
    Ok(())
}

/// Reset privacy configuration to defaults
pub fn reset_config(repo_root: Option<&Path>) -> anyhow::Result<()> {
    let (config_path, mut config) = if let Some(root) = repo_root {
        let project_config = root.join(".aiy").join("config.toml");
        let config = if project_config.exists() {
            PipelineConfig::load(&project_config)?
        } else {
            PipelineConfig::default()
        };
        (project_config, config)
    } else {
        let global_path = PipelineConfig::config_path()?;
        let config = if global_path.exists() {
            PipelineConfig::load(&global_path)?
        } else {
            PipelineConfig::default()
        };
        (global_path, config)
    };

    // Reset to defaults (preserving enabled state)
    let was_enabled = config.privacy_mode.as_ref().map(|p| p.enabled).unwrap_or(false);
    config.privacy_mode = Some(PrivacyModeConfig {
        enabled: was_enabled,
        ..Default::default()
    });

    config.save(&config_path)?;

    println!("{} Privacy configuration reset to defaults", "[OK]".green());
    println!(
        "  Config saved to: {}",
        config_path.display().to_string().dimmed()
    );

    Ok(())
}

/// Parse a boolean value from string
fn parse_bool(value: &str) -> anyhow::Result<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => anyhow::bail!(
            "Invalid boolean value: '{}'. Use true/false, 1/0, yes/no, or on/off",
            value
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(parse_bool("on").unwrap());
        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(!parse_bool("off").unwrap());
        assert!(parse_bool("invalid").is_err());
    }
}
