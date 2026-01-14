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
use url::Url;

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

    /// Invalid Ollama URL for privacy mode
    #[error("Invalid Ollama URL: {0}")]
    InvalidOllamaUrl(String),
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

    /// Privacy mode configuration (optional, default: disabled)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_mode: Option<PrivacyModeConfig>,
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
            privacy_mode: None,
        }
    }
}

// =============================================================================
// Privacy Mode Configuration (Feature: PRIV-253B)
// =============================================================================

/// Privacy mode configuration (default: OFF)
///
/// When enabled, privacy mode keeps proprietary source code strictly local while
/// leveraging cloud AI models for high-level planning. Cloud models receive only
/// redacted, opaque identifiers; all code-touching operations use a local model.
///
/// # Security
///
/// - Redaction mappings are IN-MEMORY ONLY, never persisted
/// - State files contain NO identifiers (real or opaque)
/// - Local executor URL must be loopback-only when privacy mode is enabled
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrivacyModeConfig {
    /// Enable privacy mode (default: false)
    #[serde(default)]
    pub enabled: bool,

    /// Local executor configuration
    #[serde(default)]
    pub local_executor: LocalExecutorConfig,

    /// RAG (Retrieval-Augmented Generation) settings
    #[serde(default)]
    pub rag: RagConfig,

    /// Verification loop settings
    #[serde(default)]
    pub verification: VerificationConfig,

    /// Patterns to exclude from RAG indexing
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "target/**".to_string(),
        ".git/**".to_string(),
        "*.lock".to_string(),
        "node_modules/**".to_string(),
    ]
}

impl Default for PrivacyModeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            local_executor: LocalExecutorConfig::default(),
            rag: RagConfig::default(),
            verification: VerificationConfig::default(),
            exclude_patterns: default_exclude_patterns(),
        }
    }
}

/// Local executor configuration for privacy mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalExecutorConfig {
    /// Ollama server URL (MUST be loopback-only in privacy mode)
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,

    /// Model name for code generation
    #[serde(default = "default_model")]
    pub model: String,

    /// Context window size in tokens
    #[serde(default = "default_context_size")]
    pub context_size: usize,

    /// Request timeout in milliseconds
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Temperature for generation (0.0 - 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_ollama_url() -> String {
    "http://127.0.0.1:11434".to_string()
}

fn default_model() -> String {
    "codellama:7b-instruct".to_string()
}

fn default_context_size() -> usize {
    8192
}

fn default_timeout_ms() -> u64 {
    300_000 // 5 minutes
}

fn default_temperature() -> f32 {
    0.1
}

impl Default for LocalExecutorConfig {
    fn default() -> Self {
        Self {
            ollama_url: default_ollama_url(),
            model: default_model(),
            context_size: default_context_size(),
            timeout_ms: default_timeout_ms(),
            temperature: default_temperature(),
        }
    }
}

impl LocalExecutorConfig {
    /// Validate that the Ollama URL is loopback-only (required for privacy mode)
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::InvalidOllamaUrl` if the URL is not a loopback address.
    pub fn validate_loopback_only(&self) -> Result<(), ConfigError> {
        let url = Url::parse(&self.ollama_url).map_err(|e| {
            ConfigError::InvalidOllamaUrl(format!("Invalid URL '{}': {}", self.ollama_url, e))
        })?;

        // Use url::Host for proper handling of IPv4, IPv6, and domain names
        let host = url.host().ok_or_else(|| {
            ConfigError::InvalidOllamaUrl(format!("URL '{}' has no host", self.ollama_url))
        })?;

        let is_loopback = match host {
            url::Host::Domain(domain) => domain == "localhost",
            url::Host::Ipv4(ipv4) => ipv4.is_loopback(),
            url::Host::Ipv6(ipv6) => ipv6.is_loopback(),
        };

        if !is_loopback {
            return Err(ConfigError::InvalidOllamaUrl(format!(
                "Privacy mode requires loopback-only Ollama URL. Got '{}'. \
                 Use 'http://127.0.0.1:11434' or 'http://localhost:11434'",
                self.ollama_url
            )));
        }

        Ok(())
    }
}

/// RAG (Retrieval-Augmented Generation) configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RagConfig {
    /// Max tokens for RAG context (25% of total context budget)
    #[serde(default = "default_rag_budget")]
    pub token_budget: usize,

    /// Top-k results to retrieve
    #[serde(default = "default_top_k")]
    pub top_k: usize,

    /// Minimum similarity score threshold
    #[serde(default = "default_min_similarity")]
    pub min_similarity: f32,
}

fn default_rag_budget() -> usize {
    2048
}

fn default_top_k() -> usize {
    5
}

fn default_min_similarity() -> f32 {
    0.3
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            token_budget: default_rag_budget(),
            top_k: default_top_k(),
            min_similarity: default_min_similarity(),
        }
    }
}

/// Verification loop configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationConfig {
    /// Max fmt repair attempts before giving up
    #[serde(default = "default_fmt_repairs")]
    pub max_fmt_repairs: usize,

    /// Max clippy repair attempts before giving up
    #[serde(default = "default_clippy_repairs")]
    pub max_clippy_repairs: usize,

    /// Max test repair attempts before giving up
    #[serde(default = "default_test_repairs")]
    pub max_test_repairs: usize,

    /// Global max repair attempts across all stages
    #[serde(default = "default_global_repairs")]
    pub max_global_repairs: usize,
}

fn default_fmt_repairs() -> usize {
    2
}

fn default_clippy_repairs() -> usize {
    2
}

fn default_test_repairs() -> usize {
    1
}

fn default_global_repairs() -> usize {
    8
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            max_fmt_repairs: default_fmt_repairs(),
            max_clippy_repairs: default_clippy_repairs(),
            max_test_repairs: default_test_repairs(),
            max_global_repairs: default_global_repairs(),
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

    /// Load effective configuration with precedence: Env > Project > Global > Defaults
    ///
    /// This method implements the configuration precedence order for privacy mode:
    /// 1. Environment variables (highest priority)
    /// 2. Per-project config: `<repo_root>/.aiy/config.toml`
    /// 3. Global user config: `~/.config/all-in-yum/config.toml`
    /// 4. Built-in defaults (lowest priority)
    ///
    /// # Arguments
    ///
    /// * `repo_root` - Optional repository root path for per-project config lookup
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use aiy_core::config::PipelineConfig;
    /// use std::path::Path;
    ///
    /// // Load with project-specific config if available
    /// let config = PipelineConfig::load_effective(Some(Path::new("/path/to/repo")))?;
    ///
    /// // Or load without project context (global/defaults only)
    /// let config = PipelineConfig::load_effective(None)?;
    /// ```
    pub fn load_effective(repo_root: Option<&std::path::Path>) -> Result<Self, ConfigError> {
        // Try per-project config first
        let mut config = if let Some(root) = repo_root {
            let project_config = root.join(".aiy").join("config.toml");
            if project_config.exists() {
                Self::load(&project_config)?
            } else {
                // Try global config
                let global_path = Self::config_path()?;
                if global_path.exists() {
                    Self::load(&global_path)?
                } else {
                    Self::default()
                }
            }
        } else {
            // No repo context - try global, fall back to defaults
            let global_path = Self::config_path()?;
            if global_path.exists() {
                Self::load(&global_path)?
            } else {
                Self::default()
            }
        };

        // Apply environment variable overrides
        config.apply_env_overrides();

        // Validate privacy mode constraints if enabled
        if let Some(ref privacy_config) = config.privacy_mode {
            if privacy_config.enabled {
                privacy_config.local_executor.validate_loopback_only()?;
            }
        }

        Ok(config)
    }

    /// Apply environment variable overrides to the configuration
    ///
    /// Environment variables use the `AIY_` prefix:
    /// - `AIY_PRIVACY_MODE`: Enable privacy mode ("1" or "true")
    /// - `AIY_OLLAMA_URL`: Override Ollama URL
    /// - `AIY_OLLAMA_MODEL`: Override model name
    /// - `AIY_PRIVACY_CONTEXT_SIZE`: Override context window size
    /// - `AIY_PRIVACY_RAG_BUDGET`: Override RAG token budget
    /// - `AIY_PRIVACY_MAX_REPAIRS`: Override global repair limit
    fn apply_env_overrides(&mut self) {
        // AIY_PRIVACY_MODE: Enable/disable privacy mode
        if let Ok(val) = std::env::var("AIY_PRIVACY_MODE") {
            let enabled = val == "1" || val.to_lowercase() == "true";
            if enabled {
                // Ensure privacy_mode config exists
                if self.privacy_mode.is_none() {
                    self.privacy_mode = Some(PrivacyModeConfig::default());
                }
                if let Some(ref mut privacy) = self.privacy_mode {
                    privacy.enabled = true;
                }
            } else if let Some(ref mut privacy) = self.privacy_mode {
                privacy.enabled = false;
            }
        }

        // AIY_OLLAMA_URL: Override Ollama URL
        if let Ok(url) = std::env::var("AIY_OLLAMA_URL") {
            if self.privacy_mode.is_none() {
                self.privacy_mode = Some(PrivacyModeConfig::default());
            }
            if let Some(ref mut privacy) = self.privacy_mode {
                privacy.local_executor.ollama_url = url;
            }
        }

        // AIY_OLLAMA_MODEL: Override model name
        if let Ok(model) = std::env::var("AIY_OLLAMA_MODEL") {
            if self.privacy_mode.is_none() {
                self.privacy_mode = Some(PrivacyModeConfig::default());
            }
            if let Some(ref mut privacy) = self.privacy_mode {
                privacy.local_executor.model = model;
            }
        }

        // AIY_PRIVACY_CONTEXT_SIZE: Override context window size
        if let Ok(size_str) = std::env::var("AIY_PRIVACY_CONTEXT_SIZE") {
            if let Ok(size) = size_str.parse::<usize>() {
                if self.privacy_mode.is_none() {
                    self.privacy_mode = Some(PrivacyModeConfig::default());
                }
                if let Some(ref mut privacy) = self.privacy_mode {
                    privacy.local_executor.context_size = size;
                }
            }
        }

        // AIY_PRIVACY_RAG_BUDGET: Override RAG token budget
        if let Ok(budget_str) = std::env::var("AIY_PRIVACY_RAG_BUDGET") {
            if let Ok(budget) = budget_str.parse::<usize>() {
                if self.privacy_mode.is_none() {
                    self.privacy_mode = Some(PrivacyModeConfig::default());
                }
                if let Some(ref mut privacy) = self.privacy_mode {
                    privacy.rag.token_budget = budget;
                }
            }
        }

        // AIY_PRIVACY_MAX_REPAIRS: Override global repair limit
        if let Ok(repairs_str) = std::env::var("AIY_PRIVACY_MAX_REPAIRS") {
            if let Ok(repairs) = repairs_str.parse::<usize>() {
                if self.privacy_mode.is_none() {
                    self.privacy_mode = Some(PrivacyModeConfig::default());
                }
                if let Some(ref mut privacy) = self.privacy_mode {
                    privacy.verification.max_global_repairs = repairs;
                }
            }
        }
    }

    /// Get privacy mode configuration, returning defaults if not explicitly configured
    pub fn get_privacy_mode(&self) -> PrivacyModeConfig {
        self.privacy_mode.clone().unwrap_or_default()
    }

    /// Check if privacy mode is enabled
    pub fn is_privacy_mode_enabled(&self) -> bool {
        self.privacy_mode
            .as_ref()
            .map(|p| p.enabled)
            .unwrap_or(false)
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

    // ==========================================================================
    // Privacy Mode Configuration Tests
    // ==========================================================================

    #[test]
    fn test_privacy_mode_default_disabled() {
        let config = PipelineConfig::default();
        assert!(config.privacy_mode.is_none());
        assert!(!config.is_privacy_mode_enabled());
    }

    #[test]
    fn test_privacy_mode_config_defaults() {
        let privacy = PrivacyModeConfig::default();
        assert!(!privacy.enabled);
        assert_eq!(privacy.local_executor.ollama_url, "http://127.0.0.1:11434");
        assert_eq!(privacy.local_executor.model, "codellama:7b-instruct");
        assert_eq!(privacy.local_executor.context_size, 8192);
        assert_eq!(privacy.local_executor.timeout_ms, 300_000);
        assert!((privacy.local_executor.temperature - 0.1).abs() < f32::EPSILON);
        assert_eq!(privacy.rag.token_budget, 2048);
        assert_eq!(privacy.rag.top_k, 5);
        assert!((privacy.rag.min_similarity - 0.3).abs() < f32::EPSILON);
        assert_eq!(privacy.verification.max_fmt_repairs, 2);
        assert_eq!(privacy.verification.max_clippy_repairs, 2);
        assert_eq!(privacy.verification.max_test_repairs, 1);
        assert_eq!(privacy.verification.max_global_repairs, 8);
    }

    #[test]
    fn test_local_executor_validate_loopback_127() {
        let executor = LocalExecutorConfig {
            ollama_url: "http://127.0.0.1:11434".to_string(),
            ..Default::default()
        };
        assert!(executor.validate_loopback_only().is_ok());
    }

    #[test]
    fn test_local_executor_validate_loopback_localhost() {
        let executor = LocalExecutorConfig {
            ollama_url: "http://localhost:11434".to_string(),
            ..Default::default()
        };
        assert!(executor.validate_loopback_only().is_ok());
    }

    #[test]
    fn test_local_executor_validate_loopback_ipv6() {
        let executor = LocalExecutorConfig {
            ollama_url: "http://[::1]:11434".to_string(),
            ..Default::default()
        };
        assert!(executor.validate_loopback_only().is_ok());
    }

    #[test]
    fn test_local_executor_rejects_remote_url() {
        let executor = LocalExecutorConfig {
            ollama_url: "http://192.168.1.100:11434".to_string(),
            ..Default::default()
        };
        let result = executor.validate_loopback_only();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidOllamaUrl(_)));
    }

    #[test]
    fn test_local_executor_rejects_external_domain() {
        let executor = LocalExecutorConfig {
            ollama_url: "http://ollama.example.com:11434".to_string(),
            ..Default::default()
        };
        let result = executor.validate_loopback_only();
        assert!(result.is_err());
    }

    #[test]
    fn test_privacy_mode_roundtrip() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("privacy_config.toml");

        let mut config = PipelineConfig::default();
        config.privacy_mode = Some(PrivacyModeConfig {
            enabled: true,
            local_executor: LocalExecutorConfig {
                ollama_url: "http://localhost:11434".to_string(),
                model: "codellama:13b-instruct".to_string(),
                context_size: 16384,
                timeout_ms: 600_000,
                temperature: 0.2,
            },
            rag: RagConfig {
                token_budget: 4096,
                top_k: 10,
                min_similarity: 0.5,
            },
            verification: VerificationConfig {
                max_fmt_repairs: 3,
                max_clippy_repairs: 3,
                max_test_repairs: 2,
                max_global_repairs: 12,
            },
            exclude_patterns: vec!["target/**".to_string(), ".git/**".to_string()],
        });

        config.save(&config_path).expect("Failed to save config");
        let loaded = PipelineConfig::load(&config_path).expect("Failed to load config");

        assert_eq!(config, loaded);
        assert!(loaded.is_privacy_mode_enabled());
    }

    #[test]
    fn test_backward_compatible_config_without_privacy_mode() {
        // Old config without privacy_mode section should load successfully
        let old_config_toml = r#"
enabled_agents = ["claude", "grok"]
credential_backend = "system"

[default_models]
claude = "claude-3-opus"
grok = "grok-2"

[base_urls]
claude = "https://api.anthropic.com"
grok = "https://api.x.ai"

[timeouts]
claude = 120000
grok = 120000
"#;

        let config: PipelineConfig = toml::from_str(old_config_toml).unwrap();
        assert!(config.privacy_mode.is_none());
        assert!(!config.is_privacy_mode_enabled());
        assert_eq!(config.enabled_agents, vec!["claude", "grok"]);
    }

    #[test]
    fn test_get_privacy_mode_returns_defaults_when_none() {
        let config = PipelineConfig::default();
        let privacy = config.get_privacy_mode();
        assert!(!privacy.enabled);
        assert_eq!(privacy.local_executor.ollama_url, "http://127.0.0.1:11434");
    }
}
