//! Workflow commands for privacy mode
//!
//! Provides init, execute, status, resume, and cancel commands.

use aiy_core::config::PipelineConfig;
use aiy_privacy::orchestration::{OrchestratorConfig, PrivacyOrchestrator};
use aiy_privacy::workflow::{WorkflowState, WorkflowStateName};
use colored::*;
use std::path::PathBuf;

/// Output format for workflow commands
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output for scripting
    Json,
}

/// Initialize a workflow in a directory
pub async fn init(path: Option<PathBuf>) -> anyhow::Result<()> {
    let working_dir = path.unwrap_or_else(|| {
        super::detect_repo_root().unwrap_or_else(|| std::env::current_dir().unwrap())
    });

    println!(
        "\n{} {}",
        "Initializing privacy workflow in:".cyan(),
        working_dir.display()
    );

    // Check if already initialized
    let state_path = WorkflowState::state_path(&working_dir);
    if state_path.exists() {
        let existing = WorkflowState::load(&state_path)?;
        if existing.is_active() {
            anyhow::bail!(
                "Workflow already active in this directory. Use `aiy privacy cancel` first."
            );
        }
        // Allow re-initialization if previous workflow is terminal
        println!(
            "{}",
            "Previous workflow found (terminal). Reinitializing...".yellow()
        );
    }

    // Load config
    let config = PipelineConfig::load_effective(Some(&working_dir))?;
    let privacy = config.get_privacy_mode();

    if !privacy.enabled {
        anyhow::bail!(
            "Privacy mode is not enabled. Run `aiy privacy enable` first."
        );
    }

    // Create orchestrator
    let orchestrator_config = OrchestratorConfig::from_privacy_config(&privacy, &working_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create orchestrator config: {}", e))?;

    let mut orchestrator = PrivacyOrchestrator::new(orchestrator_config);

    // Initialize (indexes directory)
    println!("{}", "Indexing directory...".dimmed());
    let stats = orchestrator
        .init()
        .await
        .map_err(|e| anyhow::anyhow!("Initialization failed: {}", e))?;

    // Save workflow state
    let mut state = WorkflowState::new();
    state.mark_ready(stats.chunks_indexed);
    state.save(&state_path)?;

    println!();
    println!("{} Workflow initialized", "[OK]".green());
    println!("  Chunks indexed: {}", stats.chunks_indexed);
    println!(
        "  State saved to: {}",
        state_path.display().to_string().dimmed()
    );

    Ok(())
}

/// Execute a request via the orchestrator
pub async fn execute(
    request: String,
    agent: String,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let working_dir = super::detect_repo_root()
        .ok_or_else(|| anyhow::anyhow!("Not in a repository. Run from a directory with .git or .aiy"))?;

    // Load and validate state
    let state_path = WorkflowState::state_path(&working_dir);
    let mut state = if state_path.exists() {
        WorkflowState::load(&state_path)?
    } else {
        anyhow::bail!("Workflow not initialized. Run `aiy privacy init` first.");
    };

    if state.state != WorkflowStateName::Ready && state.state != WorkflowStateName::Paused {
        anyhow::bail!(
            "Workflow is {:?}. Expected Ready or Paused.",
            state.state
        );
    }

    // Load config
    let config = PipelineConfig::load_effective(Some(&working_dir))?;
    let privacy = config.get_privacy_mode();

    if !privacy.enabled {
        anyhow::bail!("Privacy mode is not enabled.");
    }

    // Create orchestrator
    let orchestrator_config = OrchestratorConfig::from_privacy_config(&privacy, &working_dir)
        .map_err(|e| anyhow::anyhow!("Config error: {}", e))?;

    let mut orchestrator = PrivacyOrchestrator::new(orchestrator_config);

    // Re-initialize to index
    println!("{}", "Preparing orchestrator...".dimmed());
    let _ = orchestrator.init().await;

    // Execute
    match format {
        OutputFormat::Text => {
            println!();
            println!("{} {}", "Request:".cyan().bold(), request);
            println!("{} {}", "Agent:".cyan().bold(), agent);
            println!();
        }
        OutputFormat::Json => {}
    }

    state.mark_executing(0);
    state.save(&state_path)?;

    let result = orchestrator.execute(&request).await;

    match result {
        Ok(stats) => {
            state.update_from_stats(&stats);
            state.mark_completed();
            state.save(&state_path)?;

            match format {
                OutputFormat::Text => {
                    println!("{} Execution completed", "[OK]".green());
                    println!();
                    println!("{}", "Statistics:".white().bold());
                    println!("  Tasks completed: {}", stats.tasks_succeeded);
                    println!("  Tasks failed: {}", stats.tasks_failed);
                    println!("  Repairs attempted: {}", stats.total_repairs);
                    println!("  Duration: {}ms", stats.duration_ms);
                }
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "success": true,
                        "stats": {
                            "tasks_completed": stats.tasks_succeeded,
                            "tasks_failed": stats.tasks_failed,
                            "repairs": stats.total_repairs,
                            "duration_ms": stats.duration_ms
                        }
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
            }
        }
        Err(e) => {
            state.mark_failed(e.to_string());
            state.save(&state_path)?;

            match format {
                OutputFormat::Text => {
                    eprintln!("{} Execution failed: {}", "[ERROR]".red(), e);
                }
                OutputFormat::Json => {
                    let json = serde_json::json!({
                        "success": false,
                        "error": e.to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Show workflow status
pub fn workflow_status(format: OutputFormat) -> anyhow::Result<()> {
    let working_dir = super::detect_repo_root()
        .ok_or_else(|| anyhow::anyhow!("Not in a repository"))?;

    let state_path = WorkflowState::state_path(&working_dir);
    let state = if state_path.exists() {
        WorkflowState::load(&state_path)?
    } else {
        anyhow::bail!("No workflow found. Run `aiy privacy init` first.");
    };

    match format {
        OutputFormat::Text => {
            println!("\n{}", "Workflow Status".cyan().bold());
            println!("{}", "===============".cyan());
            println!();

            // State indicator
            let state_str = format!("{:?}", state.state);
            let state_colored = match state.state {
                WorkflowStateName::Ready => state_str.green(),
                WorkflowStateName::Executing => state_str.yellow(),
                WorkflowStateName::Paused => state_str.yellow(),
                WorkflowStateName::Completed => state_str.green().bold(),
                WorkflowStateName::Failed => state_str.red(),
                WorkflowStateName::Cancelled => state_str.red(),
                WorkflowStateName::Uninitialized => state_str.dimmed(),
            };
            println!("State: {}", state_colored);

            // Progress
            if state.total_tasks > 0 {
                let progress = state.progress_percent();
                println!(
                    "Progress: {}% ({}/{} tasks)",
                    progress, state.tasks_completed, state.total_tasks
                );
            }

            println!();
            println!("{}", "Statistics:".white().bold());
            println!("  Chunks indexed: {}", state.chunks_indexed);
            println!("  Tasks completed: {}", state.tasks_completed);
            println!("  Tasks failed: {}", state.tasks_failed);
            println!("  Repairs: {}", state.total_repairs);
            println!("  Cloud requests: {}", state.cloud_requests);
            println!("  Local executions: {}", state.local_executions);
            println!("  Duration: {}ms", state.duration_ms);

            if let Some(ref error) = state.error_message {
                println!();
                println!("{}", "Error:".red().bold());
                println!("  {}", error.red());
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&state)?;
            println!("{}", json);
        }
    }

    Ok(())
}

/// Resume a paused workflow
pub async fn resume() -> anyhow::Result<()> {
    let working_dir = super::detect_repo_root()
        .ok_or_else(|| anyhow::anyhow!("Not in a repository"))?;

    let state_path = WorkflowState::state_path(&working_dir);
    let mut state = if state_path.exists() {
        WorkflowState::load(&state_path)?
    } else {
        anyhow::bail!("No workflow found. Run `aiy privacy init` first.");
    };

    if !state.can_resume() {
        anyhow::bail!(
            "Cannot resume workflow in state {:?}. Only Paused workflows can be resumed.",
            state.state
        );
    }

    println!("{}", "Resuming workflow...".cyan());

    // Load config and create orchestrator
    let config = PipelineConfig::load_effective(Some(&working_dir))?;
    let privacy = config.get_privacy_mode();

    let orchestrator_config = OrchestratorConfig::from_privacy_config(&privacy, &working_dir)
        .map_err(|e| anyhow::anyhow!("Config error: {}", e))?;

    let mut orchestrator = PrivacyOrchestrator::new(orchestrator_config);
    let _ = orchestrator.init().await;

    state.mark_executing(state.total_tasks);
    state.save(&state_path)?;

    match orchestrator.resume().await {
        Ok(stats) => {
            state.update_from_stats(&stats);
            state.mark_completed();
            state.save(&state_path)?;

            println!("{} Workflow resumed and completed", "[OK]".green());
            println!("  Tasks completed: {}", stats.tasks_succeeded);
        }
        Err(e) => {
            state.mark_failed(e.to_string());
            state.save(&state_path)?;

            eprintln!("{} Resume failed: {}", "[ERROR]".red(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Cancel an active workflow
pub async fn cancel() -> anyhow::Result<()> {
    let working_dir = super::detect_repo_root()
        .ok_or_else(|| anyhow::anyhow!("Not in a repository"))?;

    let state_path = WorkflowState::state_path(&working_dir);
    let mut state = if state_path.exists() {
        WorkflowState::load(&state_path)?
    } else {
        anyhow::bail!("No workflow found.");
    };

    if state.is_terminal() {
        println!(
            "{} Workflow already in terminal state: {:?}",
            "[INFO]".blue(),
            state.state
        );
        return Ok(());
    }

    state.mark_cancelled();
    state.save(&state_path)?;

    println!("{} Workflow cancelled", "[OK]".green());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        let format = OutputFormat::default();
        assert!(matches!(format, OutputFormat::Text));
    }
}
