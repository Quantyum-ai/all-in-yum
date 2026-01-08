//! Security module for credential management and prompt injection defenses

mod credential_manager;
pub mod sanitization;

pub use credential_manager::{CredentialBackend, CredentialManager, MasterKey, SecurityError};
