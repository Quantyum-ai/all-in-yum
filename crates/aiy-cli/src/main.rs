//! All-in-Yum CLI - Command line interface for managing AI adapters

mod commands;
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

#[derive(Subcommand)]
pub enum Commands {
    /// Display version information
    Version,

    /// Ask a single AI agent a question
    Ask {
        /// Agent to use (e.g., grok)
        #[arg(short, long)]
        agent: String,

        /// Prompt to send to the agent (or pipe via stdin)
        #[arg(short, long)]
        prompt: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => commands::version::run(),

        Commands::Ask { agent, prompt, format } => {
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

        Commands::Review { file, agents, format } => {
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
                commands::review::ReviewResult::Blocked => {
                    // Consensus blocked the artifact
                    std::process::exit(2);
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
    }
}
