//! Review command implementation
//!
//! Provides the `aiy review <file>` command for reviewing code files using AI agents.

use crate::adapters::create_review_adapter;
use crate::registry::AGENTS;
use aiy_adapters::{AgentAdapter, AgentReview, Severity, Verdict};
use aiy_core::security::{CredentialBackend, CredentialManager};
use aiy_core::PipelineConfig;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Output format for review results
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(OutputFormat::Pretty),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!(
                "Unknown output format: {}. Use 'pretty' or 'json'",
                s
            )),
        }
    }
}

/// Arguments for the review command
pub struct ReviewArgs {
    /// File to review
    pub file: PathBuf,
    /// Agents to use (comma-separated, default: all enabled)
    pub agents: Option<String>,
    /// Output format
    pub format: OutputFormat,
}

/// Review command result indicating success or failure
#[derive(Debug)]
pub enum ReviewResult {
    /// Review completed successfully
    Success,
    /// Review failed due to no reviews being completed (SECURITY: must exit non-zero)
    NoReviews,
}

/// Run the review command
pub async fn run(args: ReviewArgs) -> anyhow::Result<ReviewResult> {
    // Validate file exists
    if !args.file.exists() {
        anyhow::bail!("File not found: {}", args.file.display());
    }

    // Read the artifact (file contents)
    let artifact = std::fs::read_to_string(&args.file)?;

    // Load configuration
    let config = PipelineConfig::load(&PipelineConfig::config_path()?)
        .unwrap_or_else(|_| PipelineConfig::default());

    // Determine which agents to use
    let requested_agents: Vec<String> = if let Some(agents_str) = &args.agents {
        agents_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        config.enabled_agents.clone()
    };

    // Validate requested agents are enabled
    for agent in &requested_agents {
        if !config.enabled_agents.contains(agent) && args.agents.is_some() {
            eprintln!(
                "{}: Agent '{}' is not enabled. Enable it with: aiy agents enable {}",
                "Warning".yellow(),
                agent,
                agent
            );
        }
    }

    // Create adapters
    let adapters = create_adapters(&config, &requested_agents).await?;

    if adapters.is_empty() {
        // Generate credential hints from registry
        let hints: Vec<String> = AGENTS
            .iter()
            .map(|a| {
                format!(
                    "aiy credentials set {:10} # for {}",
                    a.credential_provider, a.display_name
                )
            })
            .collect();
        anyhow::bail!(
            "No agents available. Set up credentials with:\n  {}",
            hints.join("\n  ")
        );
    }

    // Display progress
    println!("\n{}", "Reviewing with enabled agents...".cyan().bold());

    let agents_display: Vec<&str> = adapters.iter().map(|a| a.display_name()).collect();
    println!("Agents: {}\n", agents_display.join(", ").white());

    // Create progress bar
    let pb = ProgressBar::new(adapters.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({msg})",
            )
            .expect("Invalid progress bar template")
            .progress_chars("#>-"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    // Run reviews
    let mut reviews: Vec<AgentReview> = Vec::new();

    for adapter in &adapters {
        pb.set_message(format!("Reviewing with {}", adapter.display_name()));

        match adapter.review_artifact(&artifact).await {
            Ok(review) => {
                reviews.push(review);
            }
            Err(e) => {
                eprintln!(
                    "{} {} failed: {}",
                    "[Error]".red(),
                    adapter.display_name(),
                    e
                );
            }
        }

        pb.inc(1);
    }

    pb.finish_with_message("Complete");
    println!();

    // =========================================================================
    // SECURITY GUARD: Empty reviews must NEVER result in exit code 0
    //
    // This is a critical security check. If no reviews were completed (due to
    // agent failures, timeouts, or misconfiguration), we MUST NOT exit with
    // success. This prevents the "vacuous truth" problem where zero reviewers
    // agreeing could be misinterpreted as approval.
    //
    // Defense-in-depth: This check exists at the CLI layer in addition to
    // the ConsensusEngine checks in aiy-consensus.
    // =========================================================================
    if reviews.is_empty() {
        eprintln!(
            "\n{}: No reviews completed. Cannot determine safety.",
            "SECURITY".red().bold()
        );
        eprintln!(
            "{}",
            "All configured agents failed to produce reviews.".red()
        );
        eprintln!(
            "{}",
            "This is treated as a BLOCK to prevent unreviewed code from passing.".red()
        );
        return Ok(ReviewResult::NoReviews);
    }

    // Display results based on format
    match args.format {
        OutputFormat::Pretty => display_pretty_results(&reviews),
        OutputFormat::Json => display_json_results(&reviews)?,
    }

    Ok(ReviewResult::Success)
}

/// Create adapters based on configuration and requested agents
///
/// Iterates over the agent registry to create adapters for requested agents.
/// Uses the adapter factory to create adapters for all supported agents.
async fn create_adapters(
    config: &PipelineConfig,
    requested_agents: &[String],
) -> anyhow::Result<Vec<Box<dyn AgentAdapter>>> {
    let mut adapters: Vec<Box<dyn AgentAdapter>> = Vec::new();

    // Get credential manager
    let credential_manager = get_credential_manager(config)?;

    // Iterate over registry to create adapters for requested agents
    for agent in AGENTS {
        if !requested_agents.contains(&agent.id.to_string()) {
            continue;
        }

        // Use the adapter factory to create the adapter
        match create_review_adapter(agent.id, credential_manager.clone()).await {
            Ok(adapter) => {
                adapters.push(adapter);
            }
            Err(e) => {
                // Log the error but continue with other agents
                eprintln!(
                    "{}: Failed to create adapter for '{}': {}",
                    "Warning".yellow(),
                    agent.display_name,
                    e
                );
            }
        }
    }

    Ok(adapters)
}

/// Get credential manager based on config
fn get_credential_manager(
    config: &PipelineConfig,
) -> anyhow::Result<Arc<Mutex<CredentialManager>>> {
    // Try system keychain first if preferred
    if config.credential_backend == "system" {
        if let Ok(manager) = CredentialManager::new(CredentialBackend::SystemKeychain) {
            return Ok(Arc::new(Mutex::new(manager)));
        }
    }

    // Fallback to encrypted file
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let cred_path = config_dir.join("all-in-yum").join("credentials.enc");

    let manager = CredentialManager::new(CredentialBackend::EncryptedFile { path: cred_path })?;
    Ok(Arc::new(Mutex::new(manager)))
}

/// Display results in pretty format with colors
fn display_pretty_results(reviews: &[AgentReview]) {
    if reviews.is_empty() {
        println!("{}", "No reviews completed.".yellow());
        return;
    }

    println!("{}", "=== Review Results ===".cyan().bold());
    println!();

    let mut all_issues = Vec::new();
    let mut pass_count = 0;
    let mut total_confidence = 0.0;

    for review in reviews {
        let verdict_display = match review.verdict {
            Verdict::Pass => format!("{} Pass", "[OK]".green()),
            Verdict::Issue => format!("{} Issue", "[!]".yellow()),
            Verdict::Block => format!("{} Block", "[X]".red()),
        };

        println!(
            "{} {} (confidence: {:.0}%)",
            format!("[{}]", review.agent_id).white().bold(),
            verdict_display,
            review.confidence * 100.0
        );

        if !review.issues.is_empty() {
            for issue in &review.issues {
                let severity_color = match issue.severity {
                    Severity::Critical => "CRITICAL".red().bold(),
                    Severity::Major => "MAJOR".red(),
                    Severity::Minor => "MINOR".yellow(),
                    Severity::Nit => "NIT".white(),
                };

                println!(
                    "  - {} {}: {}",
                    severity_color, issue.category, issue.description
                );
                if let Some(location) = &issue.location {
                    println!("    Location: {}", location.dimmed());
                }
                if let Some(fix) = &issue.suggested_fix {
                    println!("    Suggested fix: {}", fix.cyan());
                }

                all_issues.push(issue.clone());
            }
        }

        if review.verdict == Verdict::Pass {
            pass_count += 1;
        }
        total_confidence += review.confidence;
    }

    println!();
    println!("{}", "=== Summary ===".cyan().bold());

    let total = reviews.len();
    let avg_confidence = if total > 0 {
        total_confidence / total as f64
    } else {
        0.0
    };

    // Determine consensus
    let consensus = if pass_count == total {
        format!("{} (unanimous)", "PASS".green().bold())
    } else if pass_count > total / 2 {
        format!("{} ({}/{} approve)", "PASS".green(), pass_count, total)
    } else if pass_count == total / 2 && total > 1 {
        format!(
            "{} ({}/{} approve)",
            "SPLIT".yellow().bold(),
            pass_count,
            total
        )
    } else {
        format!("{} ({}/{} approve)", "FAIL".red().bold(), pass_count, total)
    };

    println!("Consensus: {}", consensus);
    println!("Confidence: {:.0}% (average)", avg_confidence * 100.0);
    println!();

    // Issue summary
    if !all_issues.is_empty() {
        let critical = all_issues
            .iter()
            .filter(|i| i.severity == Severity::Critical)
            .count();
        let major = all_issues
            .iter()
            .filter(|i| i.severity == Severity::Major)
            .count();
        let minor = all_issues
            .iter()
            .filter(|i| i.severity == Severity::Minor)
            .count();
        let nit = all_issues
            .iter()
            .filter(|i| i.severity == Severity::Nit)
            .count();

        println!(
            "Issues found: {} critical, {} major, {} minor, {} nit",
            if critical > 0 {
                format!("{}", critical).red().to_string()
            } else {
                "0".to_string()
            },
            if major > 0 {
                format!("{}", major).red().to_string()
            } else {
                "0".to_string()
            },
            if minor > 0 {
                format!("{}", minor).yellow().to_string()
            } else {
                "0".to_string()
            },
            nit
        );
    } else {
        println!("Issues found: {} ", "none".green());
    }

    // Recommendation
    println!();
    if pass_count == total && all_issues.is_empty() {
        println!("{}", "Recommendation: Safe to merge".green().bold());
    } else if pass_count > total / 2 {
        println!(
            "{}",
            "Recommendation: Safe to merge with minor improvements".yellow()
        );
    } else {
        println!(
            "{}",
            "Recommendation: Address issues before merging".red().bold()
        );
    }
}

/// Display results in JSON format
fn display_json_results(reviews: &[AgentReview]) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(reviews)?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert!(matches!(
            "pretty".parse::<OutputFormat>(),
            Ok(OutputFormat::Pretty)
        ));
        assert!(matches!(
            "json".parse::<OutputFormat>(),
            Ok(OutputFormat::Json)
        ));
        assert!(matches!(
            "PRETTY".parse::<OutputFormat>(),
            Ok(OutputFormat::Pretty)
        ));
        assert!("unknown".parse::<OutputFormat>().is_err());
    }
}
