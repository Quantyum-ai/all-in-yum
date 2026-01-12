//! Pipeline configuration for the multi-agent consensus system.
//!
//! ## Security
//!
//! This configuration file must NEVER contain:
//! - API keys
//! - Passwords
//! - Tokens
//! - Any other secrets
//!
//! All sensitive credentials are managed through the credential system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// IO error during file operations
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// TOML serialization error
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// TOML deserialization error
    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    /// Configuration directory not found
    #[error("Could not determine user config directory")]
    NoConfigDir,
}

/// Pipeline configuration for the multi-agent consensus system.
///
/// This struct contains all non-sensitive configuration options for the pipeline.
///
/// # Security
///
/// This configuration must NEVER contain API keys, passwords, or other secrets.
/// All sensitive credentials are managed through the credential system in the
/// `security` module.
///
/// # Example
///
/// ```rust
/// use aiy_core::config::PipelineConfig;
///
/// let config = PipelineConfig::default();
/// assert!(config.enabled_agents.contains(&"claude".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineConfig {
    /// Which agents are enabled (e.g., ["claude", "grok", "gemini"])
    pub enabled_agents: Vec<String>,

    /// Default models per agent (e.g., {"claude": "claude-3-opus"})
    pub default_models: HashMap<String, String>,

    /// API base URLs (can override defaults)
    pub base_urls: HashMap<String, String>,

    /// Request timeouts in milliseconds
    pub timeouts: HashMap<String, u64>,

    /// Credential backend preference ("system" or "file")
    pub credential_backend: String,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        let mut default_models = HashMap::new();
        default_models.insert("claude".to_string(), "claude-3-opus-20240229".to_string());
        default_models.insert("grok".to_string(), "grok-2".to_string());
        default_models.insert("gemini".to_string(), "gemini-1.5-pro".to_string());

        let mut base_urls = HashMap::new();
        base_urls.insert(
            "claude".to_string(),
            "https://api.anthropic.com".to_string(),
        );
        base_urls.insert("grok".to_string(), "https://api.x.ai".to_string());
        base_urls.insert(
            "gemini".to_string(),
            "https://generativelanguage.googleapis.com".to_string(),
        );

        let mut timeouts = HashMap::new();
        timeouts.insert("claude".to_string(), 120_000); // 2 minutes
        timeouts.insert("grok".to_string(), 120_000);
        timeouts.insert("gemini".to_string(), 120_000);

        Self {
            enabled_agents: vec![
                "claude".to_string(),
                "grok".to_string(),
                "gemini".to_string(),
            ],
            default_models,
            base_urls,
            timeouts,
            credential_backend: "system".to_string(),
        }
    }
}

impl PipelineConfig {
    /// Returns the default configuration file path.
    ///
    /// Uses the user's config directory (e.g., `~/.config/all-in-yum/config.toml` on Linux).
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::NoConfigDir` if the user's config directory cannot be determined.
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(config_dir.join("all-in-yum").join("config.toml"))
    }

    /// Loads configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: PipelineConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Saves configuration to a TOML file.
    ///
    /// Creates parent directories if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the TOML configuration file should be saved
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = PipelineConfig::default();

        // Check enabled agents
        assert!(config.enabled_agents.contains(&"claude".to_string()));
        assert!(config.enabled_agents.contains(&"grok".to_string()));
        assert!(config.enabled_agents.contains(&"gemini".to_string()));

        // Check default models exist
        assert!(config.default_models.contains_key("claude"));
        assert!(config.default_models.contains_key("grok"));
        assert!(config.default_models.contains_key("gemini"));

        // Check base URLs exist
        assert!(config.base_urls.contains_key("claude"));
        assert!(config.base_urls.contains_key("grok"));
        assert!(config.base_urls.contains_key("gemini"));

        // Check timeouts exist
        assert!(config.timeouts.contains_key("claude"));
        assert!(config.timeouts.contains_key("grok"));
        assert!(config.timeouts.contains_key("gemini"));

        // Check credential backend default
        assert_eq!(config.credential_backend, "system");
    }

    #[test]
    fn test_save_load_roundtrip() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_config.toml");

        // Create a custom config
        let mut config = PipelineConfig {
            enabled_agents: vec!["claude".to_string(), "grok".to_string()],
            credential_backend: "file".to_string(),
            ..Default::default()
        };
        config.timeouts.insert("custom_agent".to_string(), 60_000);

        // Save it
        config.save(&config_path).expect("Failed to save config");

        // Load it back
        let loaded_config = PipelineConfig::load(&config_path).expect("Failed to load config");

        // Verify equality
        assert_eq!(config, loaded_config);
    }

    #[test]
    fn test_config_contains_no_secrets() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_config.toml");

        let config = PipelineConfig::default();
        config.save(&config_path).expect("Failed to save config");

        // Read the raw file contents
        let contents = std::fs::read_to_string(&config_path).expect("Failed to read config file");

        // Convert to lowercase for case-insensitive search
        let contents_lower = contents.to_lowercase();

        // Ensure no secret-like field names exist (actual secret values)
        // Note: "credential_backend" is a setting name, not a secret value,
        // so we check for patterns that would indicate actual secret storage
        assert!(
            !contents_lower.contains("api_key"),
            "Config should not contain api_key field"
        );
        assert!(
            !contents_lower.contains("apikey"),
            "Config should not contain apikey field"
        );
        assert!(
            !contents_lower.contains("password"),
            "Config should not contain password field"
        );
        assert!(
            !contents_lower.contains("secret"),
            "Config should not contain secret field"
        );
        assert!(
            !contents_lower.contains("token"),
            "Config should not contain token field"
        );
        assert!(
            !contents_lower.contains("auth_key"),
            "Config should not contain auth_key field"
        );
        assert!(
            !contents_lower.contains("private_key"),
            "Config should not contain private_key field"
        );

        // Verify the struct doesn't serialize any key-like patterns
        // (e.g., "sk-", "xai-", etc.)
        assert!(
            !contents.contains("sk-"),
            "Config should not contain API key prefixes"
        );
        assert!(
            !contents.contains("xai-"),
            "Config should not contain API key prefixes"
        );
    }

    #[test]
    fn test_config_path_uses_dirs() {
        // This test verifies that config_path uses dirs::config_dir
        let result = PipelineConfig::config_path();

        // The result should be Ok if dirs::config_dir() works on this system
        // or Err(NoConfigDir) if not
        match result {
            Ok(path) => {
                assert!(path.ends_with("all-in-yum/config.toml"));
                // Verify it's using the system config directory
                if let Some(system_config) = dirs::config_dir() {
                    assert!(path.starts_with(system_config));
                }
            }
            Err(ConfigError::NoConfigDir) => {
                // This is acceptable on systems without a config dir
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = PipelineConfig::load("/nonexistent/path/config.toml");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::Io(_)));
    }

    #[test]
    fn test_load_invalid_toml() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("invalid.toml");

        // Write invalid TOML
        std::fs::write(&config_path, "this is not { valid toml").expect("Failed to write file");

        let result = PipelineConfig::load(&config_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::TomlDeserialize(_)
        ));
    }

    #[test]
    fn test_save_creates_parent_directories() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir
            .path()
            .join("nested")
            .join("dirs")
            .join("config.toml");

        let config = PipelineConfig::default();
        config.save(&config_path).expect("Failed to save config");

        assert!(config_path.exists());
    }
}
