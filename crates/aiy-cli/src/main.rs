//! All-in-Yum CLI - Command line interface for managing AI adapters

#![warn(missing_docs)]

pub mod adapters;
mod commands;
pub mod credential_helper;
pub mod registry;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// All-in-Yum CLI - Unified interface for AI adapter management
#[derive(Parser)]
#[command(name = "aiy")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Top-level CLI commands
#[derive(Subcommand)]
pub enum Commands {
    /// Display version information
    Version,

    /// Ask a single AI agent a question
    Ask {
        /// Agent to use (e.g., grok)
        #[arg(short, long)]
        agent: String,

        /// Prompt to send to the agent (positional argument)
        #[arg(value_name = "PROMPT")]
        prompt_positional: Option<String>,

        /// Prompt to send to the agent (flag form, takes precedence over positional)
        #[arg(short, long)]
        prompt: Option<String>,

        /// Output format
        #[arg(short = 'f', long, value_enum, default_value = "text")]
        format: AskOutputFormatArg,
    },

    /// Review a file using configured AI agents
    Review {
        /// File to review
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Agents to use (comma-separated, default: all enabled)
        #[arg(short, long)]
        agents: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "pretty")]
        format: OutputFormatArg,
    },

    /// Manage AI agents
    #[command(subcommand)]
    Agents(AgentsCommands),

    /// Manage configuration
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Manage API credentials for AI providers
    #[command(subcommand)]
    Credentials(CredentialsCommands),

    /// Privacy mode management
    #[command(subcommand)]
    Privacy(PrivacyCommands),
}

/// Output format for review results
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormatArg {
    /// Pretty-printed output with colors
    #[default]
    Pretty,
    /// JSON output for scripting
    Json,
}

/// Output format for ask results
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum AskOutputFormatArg {
    /// Plain text output
    #[default]
    Text,
    /// JSON output for scripting
    Json,
}

/// Agent management subcommands
#[derive(Subcommand)]
pub enum AgentsCommands {
    /// List available agents with status
    List,

    /// Enable an agent
    Enable {
        /// Agent name (e.g., grok, claude, gemini, codex)
        name: String,
    },

    /// Disable an agent
    Disable {
        /// Agent name (e.g., grok, claude, gemini, codex)
        name: String,
    },

    /// Show which agents have valid credentials
    Status,
}

/// Configuration management subcommands
#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Display current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., default_models.grok, timeouts.claude)
        key: String,

        /// Value to set
        value: String,
    },

    /// Show configuration file path
    Path,

    /// Reset configuration to defaults
    Reset,
}

/// Credential management subcommands
#[derive(Subcommand)]
pub enum CredentialsCommands {
    /// Show credential status for all providers
    Status,

    /// Set credentials for a provider
    Set {
        /// Provider name (e.g., xai, anthropic, google, openai)
        provider: String,
    },

    /// Get credentials for a provider (masked)
    Get {
        /// Provider name (e.g., xai, anthropic, google, openai)
        provider: String,
    },

    /// Delete credentials for a provider
    Delete {
        /// Provider name (e.g., xai, anthropic, google, openai)
        provider: String,
    },
}

/// Privacy mode subcommands
#[derive(Subcommand)]
pub enum PrivacyCommands {
    /// Show privacy mode status and configuration
    Status,

    /// Enable privacy mode
    Enable {
        /// Ollama server URL (default: http://127.0.0.1:11434)
        #[arg(long)]
        ollama_url: Option<String>,

        /// Model to use for local code generation
        #[arg(long)]
        model: Option<String>,
    },

    /// Disable privacy mode
    Disable,

    /// Check Ollama setup and model availability
    Check,

    /// Privacy configuration subcommands
    #[command(subcommand)]
    Config(PrivacyConfigCommands),

    /// Initialize privacy workflow in current directory
    Init {
        /// Path to initialize (default: current repository root)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Execute a request via privacy orchestrator
    Execute {
        /// Request to execute
        request: String,

        /// Agent for cloud planning (e.g., claude, grok)
        #[arg(short, long, default_value = "claude")]
        agent: String,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: WorkflowOutputFormatArg,
    },

    /// Show workflow status
    WorkflowStatus {
        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: WorkflowOutputFormatArg,
    },

    /// Resume a paused workflow
    Resume,

    /// Cancel an active workflow
    Cancel,
}

/// Output format for workflow commands
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum WorkflowOutputFormatArg {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output for scripting
    Json,
}

/// Privacy configuration subcommands
#[derive(Subcommand)]
pub enum PrivacyConfigCommands {
    /// Show privacy configuration
    Show,

    /// Set a privacy configuration value
    Set {
        /// Configuration key (e.g., model, context_size, token_budget)
        key: String,

        /// Value to set
        value: String,
    },

    /// Get a privacy configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Reset privacy configuration to defaults
    Reset,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => commands::version::run(),

        Commands::Ask {
            agent,
            prompt_positional,
            prompt,
            format,
        } => {
            // Flag takes precedence over positional argument
            let prompt = prompt.or(prompt_positional);

            let output_format = match format {
                AskOutputFormatArg::Text => commands::ask::OutputFormat::Text,
                AskOutputFormatArg::Json => commands::ask::OutputFormat::Json,
            };

            commands::ask::run(commands::ask::AskArgs {
                agent,
                prompt,
                format: output_format,
            })
            .await
        }

        Commands::Review {
            file,
            agents,
            format,
        } => {
            let output_format = match format {
                OutputFormatArg::Pretty => commands::review::OutputFormat::Pretty,
                OutputFormatArg::Json => commands::review::OutputFormat::Json,
            };

            let result = commands::review::run(commands::review::ReviewArgs {
                file,
                agents,
                format: output_format,
            })
            .await?;

            // Handle ReviewResult - non-zero exit codes for security failures
            match result {
                commands::review::ReviewResult::Success => Ok(()),
                commands::review::ReviewResult::NoReviews => {
                    // SECURITY: Exit with non-zero when no reviews completed
                    std::process::exit(1);
                }
            }
        }

        Commands::Agents(cmd) => match cmd {
            AgentsCommands::List => commands::agents::list(),
            AgentsCommands::Enable { name } => commands::agents::enable(name),
            AgentsCommands::Disable { name } => commands::agents::disable(name),
            AgentsCommands::Status => commands::agents::status(),
        },

        Commands::Config(cmd) => match cmd {
            ConfigCommands::Show => commands::config::show(),
            ConfigCommands::Set { key, value } => commands::config::set(key, value),
            ConfigCommands::Path => commands::config::path(),
            ConfigCommands::Reset => commands::config::reset(),
        },

        Commands::Credentials(cmd) => commands::credentials::run(cmd),

        Commands::Privacy(cmd) => {
            let repo_root = commands::privacy::detect_repo_root();
            match cmd {
                PrivacyCommands::Status => commands::privacy::status(repo_root.as_deref()),
                PrivacyCommands::Enable { ollama_url, model } => {
                    commands::privacy::enable(repo_root.as_deref(), ollama_url, model)
                }
                PrivacyCommands::Disable => commands::privacy::disable(repo_root.as_deref()),
                PrivacyCommands::Check => commands::privacy::check(repo_root.as_deref()).await,
                PrivacyCommands::Config(config_cmd) => match config_cmd {
                    PrivacyConfigCommands::Show => {
                        commands::privacy::show_config(repo_root.as_deref())
                    }
                    PrivacyConfigCommands::Set { key, value } => {
                        commands::privacy::set_config(repo_root.as_deref(), &key, &value)
                    }
                    PrivacyConfigCommands::Get { key } => {
                        commands::privacy::get_config(repo_root.as_deref(), &key)
                    }
                    PrivacyConfigCommands::Reset => {
                        commands::privacy::reset_config(repo_root.as_deref())
                    }
                },
                PrivacyCommands::Init { path } => {
                    commands::privacy::workflow::init(path).await
                }
                PrivacyCommands::Execute {
                    request,
                    agent,
                    format,
                } => {
                    let output_format = match format {
                        WorkflowOutputFormatArg::Text => {
                            commands::privacy::WorkflowOutputFormat::Text
                        }
                        WorkflowOutputFormatArg::Json => {
                            commands::privacy::WorkflowOutputFormat::Json
                        }
                    };
                    commands::privacy::workflow::execute(request, agent, output_format).await
                }
                PrivacyCommands::WorkflowStatus { format } => {
                    let output_format = match format {
                        WorkflowOutputFormatArg::Text => {
                            commands::privacy::WorkflowOutputFormat::Text
                        }
                        WorkflowOutputFormatArg::Json => {
                            commands::privacy::WorkflowOutputFormat::Json
                        }
                    };
                    commands::privacy::workflow::workflow_status(output_format)
                }
                PrivacyCommands::Resume => commands::privacy::workflow::resume().await,
                PrivacyCommands::Cancel => commands::privacy::workflow::cancel().await,
            }
        }
    }
}
