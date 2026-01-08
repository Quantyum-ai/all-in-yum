//! Core error types for aiy-core

use thiserror::Error;

/// Core errors that can occur in aiy-core
#[derive(Debug, Error)]
pub enum CoreError {
    /// Security-related errors
    #[error("Security error: {0}")]
    Security(#[from] crate::security::SecurityError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
