//! All-in-Yum CLI - Command line interface for managing AI adapters

mod commands;

use clap::{Parser, Subcommand};

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
    /// Manage API credentials for AI providers
    #[command(subcommand)]
    Credentials(CredentialsCommands),
}

#[derive(Subcommand)]
pub enum CredentialsCommands {
    /// Show credential status for all providers
    Status,
    /// Set credentials for a provider
    Set {
        /// Provider name (e.g., openai, anthropic, grok)
        provider: String,
    },
    /// Get credentials for a provider (masked)
    Get {
        /// Provider name (e.g., openai, anthropic, grok)
        provider: String,
    },
    /// Delete credentials for a provider
    Delete {
        /// Provider name (e.g., openai, anthropic, grok)
        provider: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Version => commands::version::run(),
        Commands::Credentials(cmd) => commands::credentials::run(cmd),
    }
}
