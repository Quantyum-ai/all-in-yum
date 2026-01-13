//! # aiy-core
//!
//! Core library for All-in-Yum multi-agent consensus pipeline.
//!
//! ## Security Features
//!
//! - **Credential Management**: Secure storage with AES-256-GCM encryption
//! - **Prompt Injection Defense**: Sanitization and validation layers
//! - **Memory Safety**: Automatic zeroization of sensitive data

pub mod config;
pub mod security;
pub mod types;

pub use config::PipelineConfig;
pub use security::{CredentialBackend, CredentialManager, MasterKey, SecurityError};
pub use types::CoreError;
