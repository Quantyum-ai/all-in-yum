//! Privacy status commands
//!
//! Provides status, enable, disable, and check commands for privacy mode.

use aiy_core::config::PipelineConfig;
use colored::*;
use std::path::Path;

/// Show privacy mode status
pub fn status(repo_root: Option<&Path>) -> anyhow::Result<()> {
    let config = PipelineConfig::load_effective(repo_root)?;
    let privacy = config.get_privacy_mode();

    println!("\n{}", "Privacy Mode Status".cyan().bold());
    println!("{}", "===================".cyan());
    println!();

    // Status indicator
    let status_indicator = if privacy.enabled {
        "[ENABLED]".green().bold()
    } else {
        "[DISABLED]".yellow().bold()
    };
    println!("Status: {}", status_indicator);
    println!();

    // Local executor configuration
    println!("{}", "Local Executor:".white().bold());
    println!("  Ollama URL: {}", privacy.local_executor.ollama_url.dimmed());
    println!("  Model: {}", privacy.local_executor.model.dimmed());
    println!(
        "  Context size: {} tokens",
        privacy.local_executor.context_size.to_string().dimmed()
    );
    println!(
        "  Timeout: {}ms",
        privacy.local_executor.timeout_ms.to_string().dimmed()
    );
    println!();

    // RAG configuration
    println!("{}", "RAG Settings:".white().bold());
    println!(
        "  Token budget: {} tokens",
        privacy.rag.token_budget.to_string().dimmed()
    );
    println!("  Top-k results: {}", privacy.rag.top_k.to_string().dimmed());
    println!(
        "  Min similarity: {}",
        format!("{:.2}", privacy.rag.min_similarity).dimmed()
    );
    println!();

    // Verification configuration
    println!("{}", "Verification Limits:".white().bold());
    println!(
        "  Max fmt repairs: {}",
        privacy.verification.max_fmt_repairs.to_string().dimmed()
    );
    println!(
        "  Max clippy repairs: {}",
        privacy.verification.max_clippy_repairs.to_string().dimmed()
    );
    println!(
        "  Max test repairs: {}",
        privacy.verification.max_test_repairs.to_string().dimmed()
    );
    println!(
        "  Max global repairs: {}",
        privacy.verification.max_global_repairs.to_string().dimmed()
    );
    println!();

    // Exclude patterns
    println!("{}", "Exclude Patterns:".white().bold());
    for pattern in &privacy.exclude_patterns {
        println!("  - {}", pattern.dimmed());
    }

    Ok(())
}

/// Enable privacy mode
pub fn enable(
    repo_root: Option<&Path>,
    ollama_url: Option<String>,
    model: Option<String>,
) -> anyhow::Result<()> {
    // Determine where to save the config
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

    // Enable privacy mode with overrides
    let mut privacy = config.privacy_mode.clone().unwrap_or_default();
    privacy.enabled = true;

    if let Some(url) = ollama_url {
        privacy.local_executor.ollama_url = url;
    }

    if let Some(m) = model {
        privacy.local_executor.model = m;
    }

    // Validate loopback-only constraint
    privacy.local_executor.validate_loopback_only()?;

    config.privacy_mode = Some(privacy.clone());
    config.save(&config_path)?;

    println!(
        "{} Privacy mode {}",
        "[OK]".green(),
        "enabled".green().bold()
    );
    println!(
        "  Ollama URL: {}",
        privacy.local_executor.ollama_url.dimmed()
    );
    println!("  Model: {}", privacy.local_executor.model.dimmed());
    println!(
        "  Config saved to: {}",
        config_path.display().to_string().dimmed()
    );

    Ok(())
}

/// Disable privacy mode
pub fn disable(repo_root: Option<&Path>) -> anyhow::Result<()> {
    // Determine where to save the config
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

    // Disable privacy mode (preserve other settings)
    if let Some(ref mut privacy) = config.privacy_mode {
        privacy.enabled = false;
    }

    config.save(&config_path)?;

    println!(
        "{} Privacy mode {}",
        "[OK]".green(),
        "disabled".yellow().bold()
    );
    println!(
        "  Config saved to: {}",
        config_path.display().to_string().dimmed()
    );

    Ok(())
}

/// Check Ollama setup and model availability
pub async fn check(repo_root: Option<&Path>) -> anyhow::Result<()> {
    let config = PipelineConfig::load_effective(repo_root)?;
    let privacy = config.get_privacy_mode();

    println!("\n{}", "Privacy Mode Check".cyan().bold());
    println!("{}", "==================".cyan());
    println!();

    // Check 1: Privacy mode enabled
    let enabled_status = if privacy.enabled {
        "[PASS]".green()
    } else {
        "[WARN]".yellow()
    };
    println!(
        "{} Privacy mode: {}",
        enabled_status,
        if privacy.enabled { "enabled" } else { "disabled" }
    );

    // Check 2: Loopback-only URL
    let loopback_result = privacy.local_executor.validate_loopback_only();
    let loopback_status = if loopback_result.is_ok() {
        "[PASS]".green()
    } else {
        "[FAIL]".red()
    };
    println!(
        "{} Ollama URL loopback-only: {}",
        loopback_status, privacy.local_executor.ollama_url
    );
    if let Err(e) = loopback_result {
        println!("       {}", e.to_string().red());
    }

    // Check 3: Ollama connectivity
    println!();
    println!("{}", "Ollama Connectivity:".white().bold());

    #[cfg(feature = "http")]
    {
        use aiy_adapter_ollama::OllamaClient;
        use std::time::Duration;

        let timeout = Duration::from_millis(privacy.local_executor.timeout_ms);
        match OllamaClient::new_with_http_timeout(timeout) {
            Ok(client) => {
                let client = client.with_base_url(&privacy.local_executor.ollama_url);

                // Health check
                match client.health_check().await {
                    Ok(models) => {
                        println!("  {} Ollama server reachable", "[PASS]".green());
                        println!("  {} Available models:", "[INFO]".blue());
                        for model in &models {
                            let indicator = if model == &privacy.local_executor.model {
                                "*".green()
                            } else {
                                " ".normal()
                            };
                            println!("    {} {}", indicator, model);
                        }

                        // Check 4: Configured model available
                        let model_available = models.contains(&privacy.local_executor.model);
                        if model_available {
                            println!(
                                "  {} Configured model '{}' is available",
                                "[PASS]".green(),
                                privacy.local_executor.model
                            );
                        } else {
                            println!(
                                "  {} Configured model '{}' not found",
                                "[FAIL]".red(),
                                privacy.local_executor.model
                            );
                            println!(
                                "       Run: ollama pull {}",
                                privacy.local_executor.model.cyan()
                            );
                        }
                    }
                    Err(e) => {
                        println!("  {} Ollama server unreachable", "[FAIL]".red());
                        println!("       {}", e.to_string().red());
                        println!("       Make sure Ollama is running: {}", "ollama serve".cyan());
                    }
                }
            }
            Err(e) => {
                println!("  {} Failed to create Ollama client", "[FAIL]".red());
                println!("       {}", e.to_string().red());
            }
        }
    }

    #[cfg(not(feature = "http"))]
    {
        println!(
            "  {} HTTP feature not enabled - cannot check connectivity",
            "[SKIP]".yellow()
        );
        println!(
            "       Build with: cargo build --features http"
        );
    }

    Ok(())
}

/// Detect repository root by walking up from current directory
///
/// Looks for `.git` directory or `.aiy` directory to identify repo root.
pub fn detect_repo_root() -> Option<std::path::PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let mut path = current_dir.as_path();

    loop {
        // Check for .git directory
        if path.join(".git").exists() {
            return Some(path.to_path_buf());
        }
        // Check for .aiy directory
        if path.join(".aiy").exists() {
            return Some(path.to_path_buf());
        }

        // Move to parent
        path = path.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_repo_root_with_git() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();

        // Change to temp dir and test
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = detect_repo_root();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp.path());

        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn test_detect_repo_root_with_aiy() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join(".aiy")).unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = detect_repo_root();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp.path());

        std::env::set_current_dir(original).unwrap();
    }
}
